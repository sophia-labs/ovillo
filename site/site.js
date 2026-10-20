// Ovillo site: mount the live bench, then build the listening room from sounds/manifest.json.
import { mountBench } from "./bench/bench.js";

const benchEl = document.getElementById("bench-mount");
try {
  window.ovilloBench = mountBench(benchEl);
} catch (err) {
  benchEl.textContent = `THE BENCH COULD NOT START: ${err && err.message ? err.message : err}`;
  benchEl.className = "warn";
}

const GROUPS = [
  ["today", "MADE FOR THIS PAGE"],
  ["repo", "FROM THE REPOSITORY"],
  ["bench", "THE BENCH'S GUITARIST, OFFLINE"],
];

const fmtTime = (s) => `${Math.floor(s / 60)}:${String(Math.floor(s % 60)).padStart(2, "0")}`;
const sci = (x) => (x === 0 ? "0" : Number(x).toExponential(2).replace("e", "E"));

let current = null; // { audio, el }

function stopCurrent() {
  if (!current) return;
  current.audio.pause();
  current.el.classList.remove("playing");
  current.el.querySelector(".play").textContent = "PLAY";
  current = null;
}

function ledgerTable(p) {
  const rows = [];
  if (p.ledger) {
    for (const w of p.ledger) {
      const touches = `${w.plucks} PLUCKS · ${w.hammerOns} HO · ${w.pullOffs} PO · ${w.slides} SL`;
      rows.push(["WORLD", w.instrument.toUpperCase() + " · " + touches, ""]);
      rows.push(["SOURCE WORK (HANDS IN)", sci(w.sourceWork), ""]);
      rows.push(["CONTROL WORK (FRETTING HAND)", sci(w.controlWork), ""]);
      rows.push(["INTERNAL LOSS", sci(w.internalLoss), ""]);
      rows.push(["RADIATED", sci(w.radiation), ""]);
      rows.push(["CLOSURE RESIDUAL", sci(w.cumulativeClosureResidual), "ok"]);
    }
  }
  if (p.bench) {
    if (p.bench.patch) rows.push(["PATCH", String(p.bench.patch).toUpperCase(), ""]);
    if (p.bench.closureResidual) rows.push(["CLOSURE RESIDUAL", p.bench.closureResidual.toUpperCase(), "ok"]);
    if (p.bench.safetyLimitedFrames != null) rows.push(["SAFETY-LIMITED FRAMES", String(p.bench.safetyLimitedFrames), p.bench.safetyLimitedFrames ? "warn" : ""]);
  }
  rows.push(["WORLD LEVEL", `PEAK ${p.worldPeakDbfs} DBFS · RMS ${p.worldRmsDbfs} DBFS`, ""]);
  rows.push(["PAGE GAIN (AFTER THE WORLD)", `${p.pageGainDb > 0 ? "+" : ""}${p.pageGainDb} DB`, ""]);
  const t = document.createElement("table");
  for (const [k, v, cls] of rows) {
    const tr = t.insertRow();
    tr.insertCell().textContent = k;
    const c = tr.insertCell();
    c.textContent = v;
    c.className = "v" + (cls ? " " + cls : "");
  }
  const wrap = document.createElement("div");
  wrap.className = "scroll";
  wrap.append(t);
  return wrap;
}

function figure(src, caption) {
  const f = document.createElement("figure");
  const img = document.createElement("img");
  img.loading = "lazy";
  img.src = src;
  img.alt = caption;
  const c = document.createElement("figcaption");
  c.textContent = caption;
  f.append(img, c);
  return f;
}

function piece(p) {
  const el = document.getElementById("piece").content.firstElementChild.cloneNode(true);
  el.dataset.tags = p.tags.join(" ") + (p.ledger || p.bench ? " world" : "");
  el.querySelector(".title").textContent = p.title;
  el.querySelector(".engine").textContent = p.engine.toUpperCase();
  el.querySelector(".tags").append(...p.tags.map((t) => Object.assign(document.createElement("span"), { textContent: t.toUpperCase(), className: t })));
  el.querySelector(".note").textContent = p.note;
  const time = el.querySelector(".time");
  time.textContent = fmtTime(p.seconds);
  const figs = el.querySelector(".figs");
  figs.append(figure(p.wave, "SPECTRUM, A WATERFALL OF 36 LINE TRACES (LOG HZ)"));
  if (p.energySvg) figs.append(figure(p.energySvg, "STORED ENERGY BY BODY, EVERY 10 MS"));
  el.querySelector(".facts").append(ledgerTable(p));
  const links = el.querySelector(".links");
  const more = document.createElement("button");
  more.textContent = "TRACES AND LEDGER";
  more.addEventListener("click", () => el.classList.toggle("open"));
  links.append(more);
  if (p.score) links.append(Object.assign(document.createElement("a"), { href: p.score, textContent: "SCORE" }));
  if (p.receipt) links.append(Object.assign(document.createElement("a"), { href: p.receipt, textContent: "RECEIPT" }));
  links.append(Object.assign(document.createElement("a"), { href: p.audio, textContent: "MP3", download: "" }));

  const audio = new Audio();
  audio.preload = "none";
  audio.src = p.audio;
  const pos = el.querySelector(".pos");
  audio.addEventListener("timeupdate", () => {
    pos.style.width = `${(100 * audio.currentTime) / (audio.duration || p.seconds)}%`;
    time.textContent = `${fmtTime(audio.currentTime)} / ${fmtTime(p.seconds)}`;
  });
  audio.addEventListener("ended", () => { stopCurrent(); pos.style.width = "0"; time.textContent = fmtTime(p.seconds); });
  el.querySelector(".play").addEventListener("click", () => {
    if (current && current.el === el) { stopCurrent(); return; }
    stopCurrent();
    current = { audio, el };
    // A STOP before the mp3 has buffered rejects play(); that is not an error.
    audio.play().catch(() => {});
    el.classList.add("playing");
    el.querySelector(".play").textContent = "STOP";
  });
  el.querySelector(".track").addEventListener("click", (e) => {
    const r = e.currentTarget.getBoundingClientRect();
    const frac = Math.min(1, Math.max(0, (e.clientX - r.left) / r.width));
    audio.currentTime = frac * (audio.duration || p.seconds);
    if (!current || current.el !== el) el.querySelector(".play").click();
  });
  return el;
}

async function listen() {
  const groups = document.getElementById("groups");
  let manifest;
  try {
    manifest = await (await fetch("sounds/manifest.json")).json();
  } catch {
    groups.textContent = "THE LISTENING ROOM COULD NOT LOAD.";
    return;
  }
  document.getElementById("listen-count").textContent = `${manifest.length} PIECES`;
  for (const [id, label] of GROUPS) {
    const items = manifest.filter((p) => p.group === id);
    if (!items.length) continue;
    const g = document.createElement("div");
    g.className = "group";
    g.append(Object.assign(document.createElement("h3"), { className: "gt", textContent: label }));
    g.append(...items.map(piece));
    groups.append(g);
  }
  for (const b of document.querySelectorAll(".filters button")) {
    b.addEventListener("click", () => {
      document.querySelectorAll(".filters button").forEach((x) => x.classList.toggle("on", x === b));
      const f = b.dataset.f;
      for (const el of document.querySelectorAll(".piece")) el.hidden = f !== "all" && !el.dataset.tags.split(" ").includes(f);
    });
  }
}
listen();
