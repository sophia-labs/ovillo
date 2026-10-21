// The bench alone. A file, not an inline script: the site's CSP allows only its own script files.
import { mountBench } from "./bench.js";
window.ovilloBench = mountBench(document.getElementById("bench"));
