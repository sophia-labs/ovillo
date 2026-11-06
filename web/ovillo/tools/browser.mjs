// node tools/browser.mjs <url> [chrome]
// Open a page in headless Chrome over the DevTools protocol, wait until its
// document.title is PASS or FAIL (or 180 s), print the page's #out text and
// any console errors, and exit 0 on PASS. Used for test/worklet.html (the
// real AudioWorklet path) and the audition page's smoke test.
import { spawn } from "node:child_process";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const [url, chrome = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"] = process.argv.slice(2);
const port = 9300 + Math.floor(Math.random() * 500);
const profile = mkdtempSync(join(process.env.TMPDIR || tmpdir(), "ovillo-chrome-"));
const proc = spawn(chrome, ["--headless=new", "--disable-gpu", "--mute-audio", "--no-first-run", "--autoplay-policy=no-user-gesture-required", `--user-data-dir=${profile}`, `--remote-debugging-port=${port}`, "about:blank"], { stdio: "ignore" });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
let target;
for (let i = 0; i < 100 && !target; i++) {
  await sleep(200);
  try {
    target = await (await fetch(`http://127.0.0.1:${port}/json/new?${encodeURIComponent(url)}`, { method: "PUT" })).json();
  } catch (_) { /* not up yet */ }
}
if (!target) { console.error("chrome did not start"); proc.kill(); process.exit(2); }
const ws = new WebSocket(target.webSocketDebuggerUrl);
await new Promise((r) => ws.addEventListener("open", r));
let id = 0;
const pending = new Map();
const errors = [];
ws.addEventListener("message", (e) => {
  const m = JSON.parse(e.data);
  if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); }
  if (m.method === "Runtime.exceptionThrown") errors.push(m.params.exceptionDetails.exception?.description || m.params.exceptionDetails.text);
  if (m.method === "Runtime.consoleAPICalled" && ["error", "warning"].includes(m.params.type)) errors.push(m.params.args.map((a) => a.value ?? a.description).join(" "));
});
const send = (method, params = {}) => new Promise((r) => { const i = ++id; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
await send("Runtime.enable");
const evaluate = async (expression) => (await send("Runtime.evaluate", { expression, returnByValue: true })).result?.result?.value;
let title = "";
const t0 = Date.now();
while (Date.now() - t0 < 180_000) {
  title = await evaluate("document.title");
  if (title === "PASS" || title === "FAIL") break;
  await sleep(500);
}
console.log(await evaluate("document.getElementById('out') ? document.getElementById('out').textContent : document.body.innerText.slice(0, 2000)"));
if (errors.length) console.log("console:", errors.join("\n"));
console.log(`title: ${title} (${((Date.now() - t0) / 1000).toFixed(1)} s)`);
ws.close();
proc.kill();
process.exit(title === "PASS" ? 0 : 1);
