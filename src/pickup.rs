use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;

use crate::engine::Engine;

/// Messages from the background pickup watchers to the main loop.
#[derive(Debug)]
pub enum Pickup {
    Url(String),
    File(String),
    Status(String),
}

pub fn is_torrent_source(text: &str) -> bool {
    let t = text.trim();
    t.starts_with("magnet:?")
        || (t.starts_with("http://") || t.starts_with("https://")) && t.ends_with(".torrent")
}

pub fn is_torrent_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("torrent"))
        .unwrap_or(false)
}

/// Spawn clipboard poller. Reads every 1s; sends on change if it's a torrent source.
pub fn spawn_clipboard(tx: mpsc::Sender<Pickup>) {
    tokio::spawn(async move {
        let mut clipboard = match arboard::Clipboard::new() {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(Pickup::Status(format!("clipboard disabled: {e}"))).await;
                return;
            }
        };
        let mut last = String::new();
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let text = clipboard.get_text().unwrap_or_default();
            if text != last {
                last = text.clone();
                if is_torrent_source(&text) {
                    let _ = tx.send(Pickup::Url(text.trim().to_string())).await;
                }
            }
        }
    });
}

/// Spawn folder watcher. Sends Pickup::File for each new .torrent.
pub fn spawn_folder_watch(folder: String, tx: mpsc::Sender<Pickup>) {
    tokio::spawn(async move {
        use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
        let (inner_tx, mut inner_rx) = mpsc::channel(32);
        let mut watcher = match RecommendedWatcher::new(
            move |res| {
                if let Ok(ev) = res {
                    let _ = inner_tx.blocking_send(ev);
                }
            },
            Config::default(),
        ) {
            Ok(w) => w,
            Err(e) => {
                let _ = tx.send(Pickup::Status(format!("watch folder error: {e}"))).await;
                return;
            }
        };
        if watcher.watch(std::path::Path::new(&folder), RecursiveMode::NonRecursive).is_err() {
            let _ = tx
                .send(Pickup::Status(format!("cannot watch {folder}")))
                .await;
            return;
        }
        while let Some(ev) = inner_rx.recv().await {
            for p in ev.paths {
                if is_torrent_file(&p) {
                    let _ = tx.send(Pickup::File(p.to_string_lossy().to_string())).await;
                }
            }
        }
    });
}

/// Run a pickup through the engine, report status.
pub async fn handle(engine: Arc<Engine>, p: Pickup) -> Pickup {
    match p {
        Pickup::Url(u) => match engine.add_url(&u).await {
            Ok(()) => Pickup::Status(format!("added url: {u}")),
            Err(e) => Pickup::Status(format!("url error: {e}")),
        },
        Pickup::File(f) => match engine.add_file(&f).await {
            Ok(()) => Pickup::Status(format!("added file: {f}")),
            Err(e) => Pickup::Status(format!("file error: {e}")),
        },
        s => s,
    }
}
