import initWasm, { AriadneWorldViewer } from "./pkg/ariadne_world_web.js";

const DEFAULT_MANIFEST = "./demo/manifest.json";
const STATUS_CAUGHT_UP = 1;
const STATUS_PRESENTED = 2;

const elements = {
  canvas: document.querySelector("#world"),
  audio: document.querySelector("#audio"),
  loadDefault: document.querySelector("#load-default"),
  files: document.querySelector("#bundle-files"),
  message: document.querySelector("#message"),
  gpu: document.querySelector("#gpu-status"),
  clock: document.querySelector("#clock-status"),
  worldDigest: document.querySelector("#world-digest"),
  observationDigest: document.querySelector("#observation-digest"),
  opticalDigest: document.querySelector("#optical-digest"),
  runDigest: document.querySelector("#run-digest"),
  pending: document.querySelector("#pending-steps"),
  closure: document.querySelector("#closure"),
  receipt: document.querySelector("#receipt"),
};

let viewer = null;
let manifest = null;
let audioUrl = null;
let resumeAfterCatchUp = false;
let wasmReady = false;

function setMessage(message, error = false) {
  elements.message.textContent = message;
  elements.message.classList.toggle("error", error);
}

function splitTick(value) {
  const tick = BigInt(Math.max(0, Math.round(Number.isFinite(value) ? value : 0)));
  return [Number(tick & 0xffff_ffffn), Number((tick >> 32n) & 0xffff_ffffn)];
}

function resizeCanvas() {
  const scale = Math.min(window.devicePixelRatio || 1, 2);
  const width = Math.max(1, Math.round(elements.canvas.clientWidth * scale));
  const height = Math.max(1, Math.round(elements.canvas.clientHeight * scale));
  if (elements.canvas.width !== width) elements.canvas.width = width;
  if (elements.canvas.height !== height) elements.canvas.height = height;
  return [width, height];
}

async function sha256(bytes) {
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  const hex = [...new Uint8Array(digest)]
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
  return `sha256:${hex}`;
}

async function fetchBytes(url) {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`${url}: HTTP ${response.status}`);
  return response.arrayBuffer();
}

function decodeText(bytes) {
  return new TextDecoder().decode(bytes);
}

async function verifyFile(label, bytes, expected) {
  const observed = await sha256(bytes);
  if (observed !== expected) {
    throw new Error(`${label} digest mismatch: expected ${expected}, observed ${observed}`);
  }
}

async function loadBundleParts(nextManifest, parts, label) {
  if (nextManifest.schema !== "ariadne-world-audiovisual-bundle-v1") {
    throw new Error(`unsupported bundle schema ${nextManifest.schema}`);
  }
  const required = [
    "audio",
    "trace",
    "worldPlan",
    "listener",
    "observationProgram",
    "observationSchema",
    "opticalPlan",
    "intervention",
    "runIdentity",
  ];
  for (const name of required) {
    if (!parts[name]) throw new Error(`bundle is missing ${name}`);
    await verifyFile(name, parts[name], nextManifest.digests[name]);
  }
  const runIdentity = JSON.parse(decodeText(parts.runIdentity));
  if (runIdentity.schema !== "ariadne-run-identity-v1") {
    throw new Error(`unsupported run identity ${runIdentity.schema}`);
  }
  const identityFields = [
    "physicalWorldDigest",
    "listenerDigest",
    "interventionDigest",
    "observationProgramDigest",
    "observationSchemaDigest",
    "opticalConstitutionDigest",
    "sampleRateHz",
    "observationRateHz",
    "sourceFrames",
    "tailFrames",
    "horizonTicks",
  ];
  for (const field of identityFields) {
    if (runIdentity[field] !== nextManifest[field]) {
      throw new Error(`run identity and manifest disagree on ${field}`);
    }
  }

  const [width, height] = resizeCanvas();
  elements.canvas.width = width;
  elements.canvas.height = height;
  viewer = await AriadneWorldViewer.create(
    elements.canvas,
    decodeText(parts.worldPlan),
    decodeText(parts.listener),
    decodeText(parts.intervention),
    decodeText(parts.observationProgram),
    decodeText(parts.observationSchema),
    decodeText(parts.opticalPlan),
    decodeText(parts.runIdentity),
    new Uint8Array(parts.trace),
    nextManifest.runDigest,
  );
  if (viewer.physical_world_digest !== nextManifest.physicalWorldDigest) {
    throw new Error("viewer and manifest disagree on physical World identity");
  }
  if (viewer.observation_schema_digest !== nextManifest.observationSchemaDigest) {
    throw new Error("viewer and manifest disagree on observation identity");
  }
  if (viewer.optical_constitution_digest !== nextManifest.opticalConstitutionDigest) {
    throw new Error("viewer and manifest disagree on optical constitution identity");
  }

  if (audioUrl) URL.revokeObjectURL(audioUrl);
  audioUrl = URL.createObjectURL(new Blob([parts.audio], { type: "audio/wav" }));
  elements.audio.src = audioUrl;
  elements.audio.load();
  manifest = nextManifest;
  resumeAfterCatchUp = false;

  elements.worldDigest.textContent = nextManifest.physicalWorldDigest;
  elements.observationDigest.textContent = nextManifest.observationSchemaDigest;
  elements.opticalDigest.textContent = nextManifest.opticalConstitutionDigest;
  elements.runDigest.textContent = nextManifest.runDigest;
  elements.closure.textContent = Number(
    nextManifest.finalReceipt.globalClosureResidual,
  ).toExponential(3);
  elements.receipt.textContent = JSON.stringify(
    {
      manifest: nextManifest,
      capability: JSON.parse(viewer.capability_json),
      runIdentity,
    },
    null,
    2,
  );
  elements.gpu.textContent = "Rust/WASM · wgpu/WebGPU · fixed optical clock";
  setMessage(
    `${label}: ${(nextManifest.horizonTicks / nextManifest.sampleRateHz).toFixed(3)} s · ` +
      `${nextManifest.observationRateHz} Hz optical constitution`,
  );
}

async function loadDefaultBundle() {
  setMessage("Loading canonical Ariadne World bundle…");
  const manifestResponse = await fetch(DEFAULT_MANIFEST);
  if (!manifestResponse.ok) {
    throw new Error(
      `demo bundle unavailable (HTTP ${manifestResponse.status}); generate it with ariadne-world-export`,
    );
  }
  const nextManifest = await manifestResponse.json();
  const base = new URL(DEFAULT_MANIFEST, window.location.href);
  const entries = await Promise.all(
    Object.entries(nextManifest.files).map(async ([name, relative]) => [
      name,
      await fetchBytes(new URL(relative, base)),
    ]),
  );
  await loadBundleParts(nextManifest, Object.fromEntries(entries), "generated bundle");
}

async function loadSelectedFiles(fileList) {
  const files = [...fileList];
  const manifestFile = files.find((file) => file.name === "manifest.json");
  if (!manifestFile) throw new Error("select manifest.json and every file named by it");
  const nextManifest = JSON.parse(await manifestFile.text());
  const byName = new Map(files.map((file) => [file.name, file]));
  const parts = {};
  for (const [name, relative] of Object.entries(nextManifest.files || {})) {
    const file = byName.get(relative.split("/").at(-1));
    if (!file) throw new Error(`selected bundle is missing ${relative}`);
    parts[name] = await file.arrayBuffer();
  }
  await loadBundleParts(nextManifest, parts, "local bundle");
}

function renderFrame() {
  if (viewer && manifest) {
    const [width, height] = resizeCanvas();
    const seconds = Math.min(
      Number.isFinite(elements.audio.currentTime) ? elements.audio.currentTime : 0,
      viewer.duration_seconds,
    );
    const tick = Math.min(
      manifest.horizonTicks,
      Math.round(seconds * viewer.sample_rate_hz),
    );
    try {
      const [low, high] = splitTick(tick);
      const status = viewer.render_tick(low, high, width, height, seconds);
      const caughtUp = (status & STATUS_CAUGHT_UP) !== 0;
      const presented = (status & STATUS_PRESENTED) !== 0;
      elements.pending.textContent = String(Math.round(viewer.pending_steps));
      elements.clock.textContent = `tick ${tick.toLocaleString()} · ${caughtUp ? "synchronous" : "replaying"}`;
      if (!caughtUp && !elements.audio.paused) {
        resumeAfterCatchUp = true;
        elements.audio.pause();
      } else if (caughtUp && resumeAfterCatchUp) {
        resumeAfterCatchUp = false;
        elements.audio.play().catch((error) => setMessage(error.message, true));
      }
      if (!presented) elements.gpu.textContent = "WebGPU surface temporarily unavailable";
      else elements.gpu.textContent = "Rust/WASM · wgpu/WebGPU · fixed optical clock";
    } catch (error) {
      setMessage(error.message, true);
      viewer = null;
    }
  }
  requestAnimationFrame(renderFrame);
}

async function main() {
  await initWasm();
  wasmReady = true;
  resizeCanvas();
  window.addEventListener("resize", resizeCanvas);
  elements.loadDefault.addEventListener("click", () => {
    loadDefaultBundle().catch((error) => setMessage(error.message, true));
  });
  elements.files.addEventListener("change", () => {
    loadSelectedFiles(elements.files.files).catch((error) => setMessage(error.message, true));
  });
  elements.audio.addEventListener("seeking", () => {
    if (!viewer) return;
    resumeAfterCatchUp = resumeAfterCatchUp || !elements.audio.paused;
    elements.audio.pause();
    viewer.reset_observer();
    });
  elements.audio.addEventListener("emptied", () => {
    if (viewer) viewer.reset_observer();
    });
  requestAnimationFrame(renderFrame);
  loadDefaultBundle().catch((error) => setMessage(error.message, true));
}

main().catch((error) => {
  elements.gpu.textContent = wasmReady ? "viewer unavailable" : "WASM unavailable";
  setMessage(error.message, true);
  console.error(error);
});
