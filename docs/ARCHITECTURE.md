# Architecture

`tordln` is a single-binary Rust TUI. The torrent engine is a **vendored fork** of librqbit 8.1.1 (patched to expose file-selection and piece-map APIs that the upstream crate does not make public).

## Data flow

```
                 ┌─────────────┐
  clipboard ───▶│             │
  .torrent file │  pickup.rs  │  Pickup::Url / Pickup::File
  watch folder ─▶│  (pollers)  │──────────────┐
  CLI arg ──────▶│             │              │  mpsc::channel(32)
                 └─────────────┘              ▼
                                          ┌──────────┐
                                          │  main.rs │  event loop (100 ms poll)
                                          │  loop {} │  ├─ engine.list() refresh
                 ┌──────────────┐          │          │  ├─ handle_app_key()
                 │   engine.rs  │◀─add/del─┤          │  └─ app.draw(f)
  librqbit ─────▶│  Engine wraps│          └──────────┘
  Session   ─────▶│  Session    │                │
                 └──────────────┘                ▼
                                          ┌──────────┐
                                          │   ui.rs   │  ratatui rendering
                                          │  draw()  │  (App state → Frame)
                                          └──────────┘
```

## Layers

### `main.rs` — orchestration
- Boots `tokio`, loads `Settings`, constructs `Engine`.
- Spawns background pickups (`spawn_clipboard`, `spawn_folder_watch`) into an `mpsc` channel.
- Runs the render/event loop: polls events every 100 ms, refreshes torrent lists from the engine, drains pickups into a `Pending` source, draws `App`, and routes keys.
- `handle_app_key(app, key, engine, pending) -> bool` returns `true` on quit.
- On exit, `restore_terminal()` leaves the alternate screen + disables raw mode and mouse capture, then clears the screen.

### `ui.rs` — presentation
- `App` holds all UI state: `sidebar_mode`, `focus`, `selected`, `home_found_files`, `details`, settings-edit state, etc.
- `draw()` lays out the outer frame, a centered content column, and a bottom legend.
- Mode renderers: `draw_home`, `draw_top` (queue), `draw_detail`, `draw_sidebar`, `draw_home_files_popup`.
- Terminal lifecycle: `init_terminal()` (raw mode + alternate screen + mouse capture) and `restore_terminal()`.

### `engine.rs` — engine wrapper
Thin async wrapper over `librqbit::Session`. Exposes `add_url`, `add_file`, `list`, `details`, `set_file_selected`, `delete`. Uses the patched `update_only_files` and `with_chunk_tracker().get_have_pieces_vec()` to drive per-file selection and the piece-map.

### `config.rs` — persistence
`Settings` (TOML) with `watch_folder`, `download_dir`, `clipboard_watch`, `global_speed_limit`. Saved to `~/.config/tordln/settings.toml`.

### `modal.rs` — AddModal
The pre-download flow: download path, speed-limit toggle + slider + numeric, file checkboxes, and Confirm/Cancel. Modal keys are consumed by `AddModal::handle_key` before app keys.

### `pickup.rs` — source detection
- `spawn_clipboard` polls the clipboard every ~1 s.
- `spawn_folder_watch` uses `notify` on `watch_folder`.
- `is_torrent_source` / `is_torrent_file` classify URLs/paths.

## Why a vendored librqbit fork?

The public `librqbit` 8.x does not expose per-file selection or a readable piece-map through its stable API. To build the detail pane's file checkboxes and piece-map, `vendor/librqbit` makes `update_only_files` / `with_chunk_tracker` public and adds `get_have_pieces_vec()`. It is pinned via `[patch.crates-io]` in `Cargo.toml`, so the workspace always builds against the patched source.

## Known limitations (v1)

- Per-torrent speed limits and full file-selection UI are partially wired (engine APIs exist; not all are surfaced in the modal).
- The pre-download modal applies the global session download directory; per-torrent path overrides are not yet plumbed end-to-end.
- Drag-drop is supported only as a `.torrent` CLI argument (terminal drag-drop depends on the host shell).
