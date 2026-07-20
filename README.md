# tordln

**T**erminal **OR**rent **D**ownload **L**oader — a fast, keyboard- and mouse-driven torrent download manager that runs entirely in your terminal.

Built with Rust, [ratatui](https://ratatui.rs) (TUI), [crossterm](https://crossterm.rs) (terminal control), and a vendored fork of [librqbit](https://github.com/ikatson/librqbit) for the BitTorrent engine.

---

## Features

- **Terminal-native UI** — a 3-pane layout (sidebar / queue / detail) with a dedicated Home screen.
- **Multiple add paths** — paste a magnet from the clipboard, drop a `.torrent` file, watch a folder, or pass a `.torrent` as a CLI argument.
- **Pre-download modal** — choose the save path, set a speed limit, and pick which files to fetch *before* anything starts. Nothing downloads until you confirm.
- **Live detail pane** — per-file selection toggles, peer counts (seeders/leechers), and an ASCII piece-map.
- **Background pickups** — clipboard polling, folder watching, and drag-drop all feed the same add-modal.
- **Cross-platform packaging** — distributed as an napi-rs multi-platform npm package (`npx tordln`).

---

## Quick start

### Prerequisites

- Rust toolchain (stable, edition 2021) — install via [rustup](https://rustup.rs).
- A terminal that supports an alternate screen + mouse (Windows Terminal, ConPTY, or any modern *nix terminal).

### Build & run (from source)

```bash
# Debug build + launch (Windows PowerShell or any shell)
cargo build
./target/debug/tordln        # or: target\debug\tordln.exe on Windows
```

Or use the bundled dev script (builds, then runs the debug binary):

```bash
npm run dev
```

### Release build

```bash
cargo build --release
./target/release/tordln
```

---

## Using tordln

### Home screen

On launch you land on **Home**, showing the `TORDLN` banner. From here:

- **Paste a magnet** — `P` reads the clipboard; if it holds a `magnet:` or `.torrent` URL it opens the pre-download modal.
- **New download** — `N` does the same (copy a magnet link first, then press `N`).
- **Found files** — if `download_dir` already contains `.torrent` files, a popup lists them; `↑`/`↓` to pick, `Enter` to open the pre-download modal for that file.

### Sidebar modes

Switch with the number keys or by clicking / arrowing the sidebar:

| Key | Mode      | What it shows                                  |
|-----|-----------|------------------------------------------------|
| `1` | Home      | Banner + quick actions                         |
| `2` | Active    | In-progress downloads                         |
| `3` | Finished  | Completed downloads                           |
| `4` | Settings  | Editable config (see below)                   |

### Navigation

- `↑` / `↓` — move selection (sidebar when focused there, list/detail when focused right).
- `Tab` — swap focus between the sidebar and the right pane.
- Mouse — click the sidebar to switch modes; click a list row to select it.
- `q` — quit (clears the terminal and restores your prompt).

### Row actions

**Active** (`2`):
- `r` — remove from list (keeps files on disk)
- `d` — delete completely (removes files)
- `Space` — toggle the selected file in the detail pane

**Finished** (`3`):
- `d` — delete completely
- `t` — remove `.torrent` metadata only (keeps files)

### Pre-download modal

Opens when a source is detected (clipboard, drop, watch folder, or `Home` file pick).

1. **Download path** — editable; defaults to `settings.download_dir`.
2. **Speed limit** — toggle + slider (1–10 MB/s) + numeric entry. Off = unlimited.
3. **File checkboxes** — pick which files to fetch.
4. **Confirm / Cancel** — `Enter` confirms and starts the download; `Esc` cancels.

> Note: per-torrent speed limits and fine-grained file selection are partially wired; the engine fork exposes the APIs but some options are not yet surfaced in the UI.

### Settings (`4`)

| Field                | Meaning                                              | Edit        |
|---------------------|------------------------------------------------------|-------------|
| `download_dir`      | Default save path                                    | `e` then type, `Enter` to save |
| `watch_folder`      | Folder auto-scanned for new `.torrent`s (toggle on/off) | `Space` toggles |
| `clipboard_watch`   | Poll clipboard for magnets every ~1s                  | `Space` toggles |
| `global_speed_limit`| Default MB/s for new torrents (`None` = unlimited)   | `e` then type, `Enter` to save |

Keys in Settings: `Space` = toggle (watch folder / clipboard watch), `e` = edit text field, type to change, `Enter` = save, `Esc` = cancel edit.

---

## Configuration

Settings persist to:

```
~/.config/tordln/settings.toml
```

Example:

```toml
watch_folder = "/home/you/Downloads/tordln-watch"
download_dir = "/home/you/Downloads/tordln"
clipboard_watch = true
global_speed_limit = 10
```

Defaults: `clipboard_watch = true`, `global_speed_limit = None` (unlimited), `download_dir = ~/Downloads/tordln`.

---

## Project layout

```
tordln/
├── src/
│   ├── main.rs      # Entry point, event loop, key handling, navigation
│   ├── ui.rs        # ratatui rendering (draw, draw_home, draw_top, draw_detail, …)
│   ├── engine.rs    # Torrent engine wrapper over patched librqbit
│   ├── config.rs    # Settings load/save (~/.config/tordln/settings.toml)
│   ├── modal.rs     # AddModal (path, speed, file pick, confirm/cancel)
│   └── pickup.rs    # Clipboard poll, folder watch, source detection
├── vendor/librqbit/ # Vendored fork (file-selection + piece-map APIs)
├── npm/             # napi-rs multi-platform packaging
├── scripts/run.js   # Launches the freshly built debug binary
├── design.md        # Design system / visual tokens
├── tokens.css       # Palette (paper, text, dim, purple accent, borders)
└── Cargo.toml
```

### Module responsibilities

- **`main.rs`** — `tokio` runtime, builds `Engine`, spawns pickups, runs the 100 ms poll/render loop, routes keys via `handle_app_key`, and restores the terminal on quit.
- **`ui.rs`** — all drawing. `App` holds UI state (`sidebar_mode`, `focus`, `selected`, `home_found_files`, …). `init_terminal` / `restore_terminal` manage raw mode, the alternate screen, and mouse capture.
- **`engine.rs`** — wraps `librqbit::Session`: `add_url`, `add_file`, `list`, `details`, `set_file_selected`, `delete`.
- **`config.rs`** — the `Settings` struct + TOML persistence.
- **`modal.rs`** — `AddModal` with path/limit/file focus traversal and confirm/cancel.
- **`pickup.rs`** — `spawn_clipboard` (1 s poll), `spawn_folder_watch` (notify), and `is_torrent_source` / `is_torrent_file` detectors.

---

## npm packaging (for distribution)

The app ships as an napi-rs multi-platform package so users can run it with `npx tordln`.

```powershell
# 1. Build the release binary
cargo build --release

# 2. Copy it into the platform packages
.\npm\build-npm.ps1

# 3. Publish each platform package, then the root
cd npm/tordln-win32-x64; npm publish --access public   # on Windows runner
cd npm/tordln-linux-x64; npm publish --access public   # on Linux runner
cd npm/tordln;           npm publish --access public
```

The root `npm/tordln` package declares `tordln-win32-x64` / `tordln-linux-x64` as `optionalDependencies` with `os`/`cpu` constraints, so `bin/tordln.mjs` resolves and `execFile`s only the matching binary with inherited stdio (the TUI owns the terminal; Ctrl-C works).

See [`npm/README.md`](npm/README.md) for the full packaging guide.

---

## Design system

`tordln` follows a locked Hallmark-style terminal design: near-black paper, white/grey text, a purple accent, 1px dim-grey hairline borders, sharp corners (`--radius: 0`), and monospace typography. See [`design.md`](design.md) and [`tokens.css`](tokens.css).

---

## Documentation

- [Keybindings](docs/KEYBINDINGS.md) — full key + mouse reference and the on-screen legend.
- [Architecture](docs/ARCHITECTURE.md) — data flow, module responsibilities, the vendored librqbit fork.
- [Design system](../design.md) — visual tokens and layout principles.

## License

See repository for license details.
