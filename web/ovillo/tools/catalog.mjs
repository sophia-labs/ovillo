// node tools/catalog.mjs <ovillo.wasm> <ovillo-catalog.json>
// Write the catalog the wasm module carries (the kind modules' own tables,
// the indices the ABI takes) to a JSON file. `cargo test -p ovillo-portable`
// checks the same file against the native build of the same code.
import { readFileSync, writeFileSync } from "node:fs";
import { OvilloCore } from "../core.js";

const [wasm, out] = process.argv.slice(2);
if (!wasm || !out) {
  console.error("usage: node tools/catalog.mjs <ovillo.wasm> <ovillo-catalog.json>");
  process.exit(2);
}
const core = await OvilloCore.load(readFileSync(wasm));
const catalog = core.catalog();
writeFileSync(out, JSON.stringify(catalog, null, 2) + "\n");
console.log(`${out}: ${catalog.kinds.length} kinds (${catalog.kinds.map((k) => k.id).join(", ")})`);
