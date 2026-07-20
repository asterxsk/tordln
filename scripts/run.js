#!/usr/bin/env node
// Runs the freshly built tordln binary, passing through CLI args.
// Used by `npm run dev`.
import { spawn } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "..");

const isWin = process.platform === "win32";
const bin = isWin ? "tordln.exe" : "tordln";
const profile = "debug"; // `npm run dev` uses the fast debug build
const binPath = path.join(root, "target", profile, bin);

if (!fs.existsSync(binPath)) {
  console.error(`[tordln] binary not found at ${binPath}\nRun: cargo build`);
  process.exit(1);
}

const child = spawn(binPath, process.argv.slice(2), { stdio: "inherit" });
child.on("exit", (code) => process.exit(code ?? 0));
