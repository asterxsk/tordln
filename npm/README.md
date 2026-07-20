# tordln — npm packaging (npx-ready)

This folder holds the **napi-rs multi-platform** npm packaging for `tordln`.
Distribution is via `npx tordln` (or a global `npm i -g tordln`).

## Layout

```
npm/
  tordln/                 root package  (tordln)
    package.json        bin + optionalDependencies on the platform pkgs
    bin/tordln.mjs       resolves the platform binary and exec's it
  tordln-win32-x64/       (tordln-win32-x64)  -> tordln.exe
  tordln-linux-x64/       (tordln-linux-x64)  -> tordln
```

The root package declares both platform packages as `optionalDependencies`
with `os`/`cpu` constraints, so npm only installs the one matching the
user's machine. `bin/tordln.mjs` locates that binary and `execFile`s it with
inherited stdio, so the TUI owns the terminal and Ctrl-C works.

## Build the binary into the packages

From the repo root (PowerShell):

```powershell
cargo build --release
.\npm\build-npm.ps1
```

`build-npm.ps1` copies `target/release/tordln.exe` into `tordln-win32-x64/`
(on Windows) or `target/release/tordln` into `tordln-linux-x64/` (on Linux).

## Publish (CI-per-platform recommended)

Each platform package is published from the matching runner:

```powershell
cd npm/tordln-win32-x64; npm publish --access public
# on linux runner:
cd npm/tordln-linux-x64; npm publish --access public
# then the root:
cd npm/tordln; npm publish --access public
```

## Before publishing

1. Verify the name is still free: `npm view tordln version`
   (404 = available). The unscoped `tordln` name is first-come-first-served,
   so claim it (even an empty publish) before someone else does.
2. Bump versions together (root + platform packages must match).

## Local test without publishing

```powershell
# from repo root, after build-npm.ps1:
cd npm/tordln; npm link
tordln            # runs the locally-linked binary
# or:
node npm/tordln/bin/tordln.mjs
```
