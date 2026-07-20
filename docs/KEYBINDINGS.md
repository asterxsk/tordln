# Keybindings

A global legend is shown at the bottom of the screen in every mode:

```
1 Home  2 Active  3 Finished  4 Settings  ↑/↓ nav  Tab focus  q quit
```

The mode-specific part appends after ` | `:

| Mode      | Legend                                              |
|-----------|-----------------------------------------------------|
| Home      | `p paste  n new  enter start`                     |
| Active    | `r remove  d delete  space file`                  |
| Finished  | `d delete  t remove .torrent`                    |
| Settings  | `space=toggle  type=edit  enter=save`             |

## Full reference

### Global
| Key       | Action                              |
|-----------|-------------------------------------|
| `1` `2` `3` `4` | Switch sidebar mode (Home / Active / Finished / Settings) |
| `Tab`     | Swap focus between sidebar and right pane |
| `↑` `↓`  | Move selection (sidebar when focused there, list/detail when focused right) |
| `q`       | Quit (clears terminal, restores prompt) |

### Home (`1`)
| Key   | Action                                                        |
|-------|---------------------------------------------------------------|
| `P`   | Paste magnet/`.torrent` URL from clipboard → pre-download modal |
| `N`   | New download from clipboard link → pre-download modal          |
| `↑` `↓` | Move selection in the found-files popup (when present)    |
| `Enter` | Open pre-download modal for the highlighted found file       |

### Active (`2`)
| Key     | Action                                |
|---------|---------------------------------------|
| `↑` `↓` | Select a download row               |
| `r`     | Remove from list (keeps files)        |
| `d`     | Delete completely (removes files)     |
| `Space` | Toggle selected file in detail pane  |

### Finished (`3`)
| Key   | Action                          |
|-------|---------------------------------|
| `↑` `↓` | Select a finished row         |
| `d`   | Delete completely              |
| `t`   | Remove `.torrent` metadata only (keeps files) |

### Settings (`4`)
| Key       | Action                                  |
|-----------|-----------------------------------------|
| `↑` `↓`   | Move between the 4 fields              |
| `Space`   | Toggle `watch_folder` / `clipboard_watch` |
| `e`       | Edit a text field (`download_dir`, `global_speed_limit`) |
| typing    | Change the focused text field          |
| `Backspace` | Delete a character in a text field |
| `Enter`   | Save settings                          |
| `Esc`     | Cancel edit                           |

### Pre-download modal
| Key        | Action                                  |
|------------|-----------------------------------------|
| `Tab`      | Move between path / limit / files / confirm / cancel |
| typing     | Edit the download path or speed value   |
| `Space`    | Toggle the speed-limit checkbox; toggle a file checkbox |
| `↑` `↓`   | Move file selection                     |
| `Enter`    | Confirm and start the download          |
| `Esc`      | Cancel                                 |

### Mouse
- Click the **sidebar** to switch modes.
- Click a **list row** to select it.
- Mouse capture is enabled on launch and disabled on quit.
