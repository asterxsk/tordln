#!/usr/bin/env node
// npx entry point for tordln.
// Resolves the platform-specific binary package and exec's it in place,
// so the TUI owns the terminal (stdio inherited) and Ctrl-C / SIGINT work.
import { execFile } from "node:child_process";
import { createRequire } from "node:module";
import fs from "node:fs";
import path from "node:path";

const require = createRequire(import.meta.url);

const SCOPE = "tordln";
const platform = `${process.platform}-${process.arch}`;

const pkgName = (() => {
  switch (platform) {
    case "win32-x64":
      return "tordln-win32-x64";
    case "linux-x64":
      return "tordln-linux-x64";
    default:
      fail(
        `tordln does not support this platform yet: ${platform}\n` +
          `Supported: win32-x64, linux-x64.`
      );
  }
})();

const binName = process.platform === "win32" ? "tordln.exe" : "tordln";

let binPath;
try {
  // Resolve the binary from the optional platform dependency.
  const pkgJsonPath = require.resolve(`${pkgName}/package.json`);
  const pkgDir = path.dirname(pkgJsonPath);
  const candidate = path.join(pkgDir, binName);
  if (!fs.existsSync(candidate)) {
    fail(`tordln binary not found at ${candidate}.\nTry reinstalling: npm i -g ${pkgName}`);
  }
  binPath = candidate;
} catch (err) {
  fail(
    `tordln platform package "${pkgName}" is not installed.\n` +
      `Install it explicitly: npm i -g ${pkgName}\n` +
      `(Under npx it is usually pulled in automatically as an optional dependency.)`
  );
}

// execFile with stdio:"inherit" + the current argv (minus node + script).
const args = process.argv.slice(2);
const child = execFile(binPath, args, { stdio: "inherit" }, (code) => {
  process.exit(typeof code === "number" ? code : 1);
});

// Forward termination signals so the TUI can clean up the terminal.
for (const sig of ["SIGINT", "SIGTERM"]) {
  process.on(sig, () => {
    if (!child.killed) child.kill(sig);
  });
}

function fail(msg) {
  console.error(`[tordln] ${msg}`);
  process.exit(1);
}
