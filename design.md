# tordln — Design System

Locked design system for `tordln`, a terminal torrent download manager.
Generated from the design discussion. Hallmark Terminal genre, black/white + purple accent.

## Principles

- Sharp corners only. `--radius: 0` everywhere. No rounding, ever.
- Hairline 1px borders, dim grey. No heavy chrome.
- Monospace-only typography (terminal aesthetic — single font is the design).
- Mouse + keyboard navigation, always. Tab swaps focus between panes.

## Layout — 3 panes

```
┌──────────┬───────────────────────────────┐
│ SIDEBAR  │  DOWNLOAD LIST (top, right)    │
│ Active   │  1 name [████▓░] 52% live      │
│ Finished │  2 name [███░░░] 31% live      │
│ Settings │                                │
│          ├───────────────────────────────┤
│          │  DETAIL PANE (bottom, right)   │
│          │  name / seeders / leechers     │
│          │  [x] file1.bin  [x] file2.bin  │
│          │  piece map: ▓▓░░▓▓░░▓▓░░       │
└──────────┴───────────────────────────────┘
```

- **Left sidebar** — modes: `Active` / `Finished` / `Settings`. Switch via `1`/`2`/`3`, mouse click, or `↑`/`↓` when focused.
- **Right top** — active download list. Inline single-line rows.
- **Right bottom** — detail of selected download: info + file checkboxes (live toggle) + piece map.

## Add flow — modal (not auto-start)

On source detected (clipboard link OR new `.torrent` in watch folder):
1. Modal pops.
2. **Download path** — editable, defaults to settings `download_dir`.
3. **Speed limit** — checkbox "Limit speed" + slider (1–10 MB/s) + free numeric box on slider's right. Unchecked = unlimited.
4. **File checkboxes** — list torrent files, pick which to fetch.
5. Confirm → torrent starts.

## Pickup paths (3)

1. **Clipboard** — polls every 1s; `magnet:` / `.torrent` URL → modal.
2. **Drag-drop** — `.torrent` path as CLI arg → modal.
3. **Folder watch** — `notify` on `watch_folder` → modal on new `.torrent`.

## Row actions

- **Active rows**: two buttons — `Remove from list` (keeps files), `Delete completely` (removes files). Key or click.
- **Finished rows**: three buttons — `Remove from list`, `Delete completely`, `Delete .torrent` (metadata only).

## Settings pane (4 fields)

- `download_dir` — default save path.
- `watch_folder` — auto-scan folder + on/off toggle.
- `clipboard_watch` — on/off.
- `global_speed_limit` — default MB/s for new torrents (reuses slider control; 0/none = unlimited).

All persisted to `~/.config/tordln/settings.toml`.

## Visual tokens

See `tokens.css`. Paper near-black, white/grey text, purple accent, dim-grey hairline borders, radius 0.
State colors: live = white, finished = purple, error/paused = dim red.

## Progress rendering

- **List rows**: inline ASCII shade bar — `[████▓░] 52%`.
- **Detail piece map**: solid block grid, one cell per piece. Purple-filled = downloaded, dim = pending.
