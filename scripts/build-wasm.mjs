// Builds the WASI web engine (crates/web) and copies it where the Vite web build
// imports it from (src/platform/web/engine/, gitignored). Node, not PowerShell, so the
// same script runs on the Ubuntu GitHub Actions runner that deploys Pages.
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const target = "wasm32-wasip1";
execFileSync("cargo", ["build", "-p", "omniconvert-web", "--target", target, "--profile", "web"], { cwd: root, stdio: "inherit" });

const from = join(root, "target", target, "web", "omniconvert-web.wasm");
const to = join(root, "src", "platform", "web", "engine", "omniconvert-web.wasm");
mkdirSync(dirname(to), { recursive: true });
copyFileSync(from, to);
console.log(`engine → ${to} (${(statSync(to).size / 1048576).toFixed(1)} MB)`);
