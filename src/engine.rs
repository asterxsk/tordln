use librqbit::{AddTorrent, Session};
use std::collections::HashSet;
use anyhow::Context;

#[derive(Debug, Clone)]
pub struct TorrentInfo {
    pub id: usize,
    pub name: String,
    pub progress: f32,
    pub state: String,
}

/// Per-file info for the detail pane.
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub size: u64,
    pub selected: bool,
    pub progress: f32,
}

/// Rich detail view for a single torrent.
#[derive(Debug, Clone)]
pub struct TorrentDetails {
    pub id: usize,
    pub name: String,
    pub progress: f32,
    pub state: String,
    pub seeders: u32,
    pub leechers: u32,
    pub files: Vec<FileInfo>,
    pub piece_map: Vec<bool>,
}

pub struct Engine {
    pub session: std::sync::Arc<Session>,
}

impl Engine {
    pub async fn new(download_dir: &str) -> anyhow::Result<Self> {
        std::fs::create_dir_all(download_dir)?;
        let session = Session::new(download_dir.into()).await?;
        Ok(Engine { session })
    }

    /// Add from a magnet link or .torrent URL.
    pub async fn add_url(&self, url: &str) -> anyhow::Result<()> {
        self.session
            .add_torrent(AddTorrent::from_url(url), None)
            .await?;
        Ok(())
    }

    /// Add from a local .torrent file path.
    pub async fn add_file(&self, path: &str) -> anyhow::Result<()> {
        self.session
            .add_torrent(AddTorrent::from_local_filename(path)?, None)
            .await?;
        Ok(())
    }

    /// Snapshot current torrents for the UI.
    pub fn list(&self) -> Vec<TorrentInfo> {
        self.session
            .with_torrents(|torrents| {
                let mut out = Vec::new();
                for (id, mt) in torrents {
                    let stats = mt.stats();
                    let progress =
                        stats.progress_bytes as f32 / (stats.total_bytes as f32).max(1.0);
                    let name = mt
                        .with_state(|s| match s {
                            librqbit::ManagedTorrentState::Live(l) => {
                                l.info().name.as_ref().map(|n| n.to_string())
                            }
                            _ => None,
                        })
                        .unwrap_or_else(|| format!("torrent#{id}"));
                    let name = if name.is_empty() {
                        format!("torrent#{id}")
                    } else {
                        name
                    };
                    out.push(TorrentInfo {
                        id,
                        name,
                        progress,
                        state: format!("{:?}", stats.state),
                    });
                }
                out
            })
    }

    /// Build a rich detail view for one torrent (files, peers, piece map).
    pub fn details(&self, id: usize) -> Option<TorrentDetails> {
        self.session
            .with_torrents(|torrents| {
                let mut mt_ref: Option<&_> = None;
                for (tid, t) in torrents {
                    if tid == id {
                        mt_ref = Some(t);
                        break;
                    }
                }
                let mt = mt_ref?;

                let stats = mt.stats();
                let progress = stats.progress_bytes as f32 / (stats.total_bytes as f32).max(1.0);
                let state = format!("{:?}", stats.state);

                // Name + file list come from the live metadata info.
                let (name, files): (String, Vec<FileInfo>) = mt
                    .with_state(|s| -> anyhow::Result<(String, Vec<FileInfo>)> {
                        match s {
                            librqbit::ManagedTorrentState::Live(l) => {
                                let info = l.info();
                                let only_files: Option<Vec<usize>> = mt.only_files();
                                let name = info
                                    .name
                                    .as_ref()
                                    .map(|n| n.to_string())
                                    .filter(|n| !n.is_empty())
                                    .unwrap_or_else(|| format!("torrent#{id}"));
                                let file_progress = &stats.file_progress;
                                let files = info
                                    .iter_file_details()
                                    .map_err(|e| anyhow::anyhow!("{e:?}"))?
                                    .enumerate()
                                    .map(|(idx, d)| {
                                        let size = d.len;
                                        let selected = only_files
                                            .as_ref()
                                            .map(|o| o.contains(&idx))
                                            .unwrap_or(true);
                                        let downloaded = file_progress
                                            .get(idx)
                                            .copied()
                                            .unwrap_or(0)
                                            .min(size);
                                        let progress = if size == 0 {
                                            1.0
                                        } else {
                                            downloaded as f32 / size as f32
                                        };
                                        Ok::<_, anyhow::Error>(FileInfo {
                                            name: d
                                                .filename
                                                .to_string()
                                                .unwrap_or_else(|_| "<INVALID NAME>".to_string()),
                                            size,
                                            selected,
                                            progress,
                                        })
                                    })
                                    .collect::<anyhow::Result<Vec<_>>>()?;
                                Ok((name, files))
                            }
                            _ => Ok((format!("torrent#{id}"), Vec::new())),
                        }
                    })
                    .unwrap_or_else(|_| (format!("torrent#{id}"), Vec::new()));

                // Peer counts from the live snapshot's aggregated peer stats.
                let (seeders, leechers) = stats
                    .live
                    .as_ref()
                    .map(|ls| {
                        let ps = &ls.snapshot.peer_stats;
                        // "seen" = total peers encountered (seeders);
                        // "live" = currently connected/active peers (leechers).
                        (ps.seen as u32, ps.live as u32)
                    })
                    .unwrap_or((0, 0));

                // Real per-piece have-bitfield, via the patched librqbit API.
                // One bool per piece: true = fully downloaded + verified.
                let piece_map: Vec<bool> = mt
                    .with_chunk_tracker(|ct| ct.get_have_pieces_vec())
                    .unwrap_or_default();

                Some(TorrentDetails {
                    id,
                    name,
                    progress,
                    state,
                    seeders,
                    leechers,
                    files,
                    piece_map,
                })
            })
    }

    /// Set the exact set of selected (downloaded) files for a torrent.
    ///
    /// `selected` is the full list of file indices that should be downloaded.
    /// Pass an empty slice to select none. The patched librqbit
    /// `Session::update_only_files` reconfigures the chunk tracker live.
    pub async fn set_selected_files(&self, id: usize, selected: &[usize]) -> anyhow::Result<()> {
        let handle = self
            .session
            .with_torrents(|ts| {
                for (tid, mt) in ts {
                    if tid == id {
                        return Some(mt.clone());
                    }
                }
                None
            })
            .context(format!("torrent {id} is not managed"))?;
        let set: HashSet<usize> = selected.iter().copied().collect();
        self.session.update_only_files(&handle, &set).await
    }

    /// Toggle whether a single file index is selected, preserving the rest.
    pub async fn set_file_selected(
        &self,
        id: usize,
        file_index: usize,
        selected: bool,
    ) -> anyhow::Result<()> {
        // Read the current selection from the torrent's `only_files`.
        let current: Vec<usize> = self
            .session
            .with_torrents(|ts| {
                for (tid, mt) in ts {
                    if tid == id {
                        return mt.only_files().unwrap_or_default();
                    }
                }
                Vec::new()
            });

        let mut set: HashSet<usize> = current.into_iter().collect();
        if selected {
            set.insert(file_index);
        } else {
            set.remove(&file_index);
        }
        self.set_selected_files(id, &set.iter().copied().collect::<Vec<_>>())
            .await
    }
}
