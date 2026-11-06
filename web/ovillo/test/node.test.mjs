// The portable path in wasm (node --test test/): every World kind rendered
// offline through ovillo.wasm, its books closed, its receipt written to
// receipts/<kind>.receipt.json and compared with the native receipt
// (ovillo/crates/ovillo-portable/receipts/); the catalog the wasm carries is
// the file's; block segmentation and other instances change nothing; a
// worklet-shaped render (128-frame quanta) is the offline render.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, writeFileSync, mkdirSync, existsSync } from "node:fs";
import { OvilloCore, renderOffline } from "../core.js";
import { Ovillo } from "../ovillo.js";
import { scenario } from "./scenarios.mjs";

const here = new URL("../", import.meta.url);
const core = await OvilloCore.load(readFileSync(new URL("ovillo.wasm", here)));
const catalog = core.catalog();
const nativeDir = new URL("../../ovillo/crates/ovillo-portable/receipts/", here);
mkdirSync(new URL("receipts/", here), { recursive: true });

test("the catalog the wasm carries is ovillo-catalog.json", () => {
  const file = JSON.parse(readFileSync(new URL("ovillo-catalog.json", here), "utf8"));
  assert.deepEqual(catalog, file);
});

for (const kind of catalog.kinds.map((k) => k.id)) {
  test(`${kind}: an offline render through the wasm closes its books`, () => {
    const r = renderOffline(core, scenario(kind, catalog));
    const l = r.receipt.ledger;
    assert.equal(r.receipt.events.refused, 0, r.receipt.events.lastError);
    assert.equal(r.receipt.events.errors, 0, r.receipt.events.lastError);
    assert.ok(r.left.every(Number.isFinite) && r.right.every(Number.isFinite));
    assert.ok(r.left.some((v) => v !== 0), "silent");
    assert.ok(l.sourceWork > 0 && l.radiation > 0);
    assert.ok(Math.abs(l.maximumFrameResidual) <= l.frameTolerance, `frame residual ${l.maximumFrameResidual} > ${l.frameTolerance}`);
    assert.ok(Math.abs(l.cumulativeClosureResidual) <= l.closureTolerance, `cumulative residual ${l.cumulativeClosureResidual} > ${l.closureTolerance}`);
    assert.ok(l.frameTolerance <= Math.max(1e-12, 1e-15 * l.throughput) * 1.000001 && l.closureTolerance <= Math.max(1e-9, 1e-12 * l.throughput) * 1.000001);
    assert.ok(l.closes);
    writeFileSync(new URL(`receipts/${kind}.receipt.json`, here), JSON.stringify(r.receipt, null, 2) + "\n");
    // The native build's receipt of the same render (cargo test -p ovillo-portable).
    const nativePath = new URL(`${kind}.receipt.json`, nativeDir);
    if (existsSync(nativePath)) {
      const n = JSON.parse(readFileSync(nativePath, "utf8"));
      assert.equal(r.receipt.frames, n.frames);
      // wasm's transcendental functions are Rust's libm, natively the platform's: a few results differ in
      // the last bit (mus-wasm's lib.rs measured it). Smooth Worlds agree to 1e-6; the race props' buzzer is a
      // self-oscillating armature behind a hysteresis position gate, so those bits move its switching instants
      // and the trajectories part (each still closes its own books): 1 %.
      const tolerance = { race_props: 1e-2 }[kind] ?? 1e-6;
      for (const key of ["sourceWork", "internalLoss", "radiation"]) {
        const [a, b] = [l[key], n.ledger[key]];
        assert.ok(Math.abs(a - b) <= tolerance * Math.max(Math.abs(a), Math.abs(b)) + 1e-300, `${key}: wasm ${a} native ${b}`);
      }
    }
  });
}

test("a worklet-shaped render (128-frame quanta) is the offline render, sample for sample", () => {
  const spec = scenario("tract", catalog);
  const a = renderOffline(core, { ...spec, seconds: 1.5 });
  const b = renderOffline(core, { ...spec, seconds: 1.5, block: 128 });
  assert.equal(a.receipt.outputDigest, b.receipt.outputDigest);
});

test("many Worlds in one module are independent", () => {
  const spec = scenario("steel-string", catalog);
  const alone = renderOffline(core, { ...spec, seconds: 1 }).receipt.outputDigest;
  const w = core.create("steel-string", { options: { score: spec.score } });
  const other = core.create("trombone", {});
  other.set("mouth", 0.3);
  let done = 0;
  while (done < 48000) {
    const n = Math.min(128, 48000 - done);
    done += w.render(n);
    other.render(128);
  }
  assert.equal(w.receipt().outputDigest, alone);
  w.dispose();
  other.dispose();
});

test("names, enums and refusals", () => {
  const w = core.create("tract", { anatomy: "soprano" });
  assert.throws(() => w.event("gesture", { name: "yodel" }), /not one of/);
  assert.throws(() => w.set("loudness", 1), /no control/);
  assert.throws(() => core.create("piano", {}, 44100), /48000/);
  w.event("sing", { pitch: 60 });
  w.render(128);
  assert.match(w.lastError(), /no pitch table/);
  w.dispose();
});

test("Ovillo.render in Node loads ovillo.wasm beside ovillo.js", async () => {
  const r = await Ovillo.render({ kind: "glass-bells", seconds: 0.5, events: [{ at: 0, name: "strike", args: { element: 1, velocity: 0.8 } }] });
  assert.equal(r.frames, 24000);
  assert.ok(r.receipt.ledger.closes);
});
