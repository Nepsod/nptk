//! Main filesystem model implementation.

use crate::filesystem::cache::FileSystemCache;
use crate::filesystem::entry::{FileEntry, FileMetadata, FileType};
use crate::filesystem::error::FileSystemError;
use crate::filesystem::icon::{IconProvider, MimeIconProvider};
use crate::filesystem::io_uring;
use crate::filesystem::watcher::{FileSystemChange, FileSystemWatcher};
use std::path::{Path, PathBuf};
use std::os::unix::fs::PermissionsExt;
use std::sync::{Arc, Mutex};
use tokio::sync::{broadcast, mpsc};

/// Events emitted by the filesystem model for UI updates.
#[derive(Debug, Clone)]
pub enum FileSystemEvent {
    /// A directory was loaded with its entries.
    DirectoryLoaded {
        /// Path to the directory.
        path: PathBuf,
        /// Entries in the directory.
        entries: Vec<FileEntry>,
    },
    /// A new entry was added.
    EntryAdded {
        /// Path to the new entry.
        path: PathBuf,
        /// The new entry.
        entry: FileEntry,
    },
    /// An entry was removed.
    EntryRemoved {
        /// Path to the removed entry.
        path: PathBuf,
    },
    /// An entry was modified.
    EntryModified {
        /// Path to the modified entry.
        path: PathBuf,
        /// The updated entry.
        entry: FileEntry,
    },
    /// An entry was renamed.
    EntryRenamed {
        /// Old path.
        old_path: PathBuf,
        /// New path.
        new_path: PathBuf,
    },
}

/// Tasks for the async worker thread.
#[derive(Debug)]
enum FileSystemTask {
    /// Load a directory.
    LoadDirectory(PathBuf),
    /// Refresh (reload) a directory.
    RefreshDirectory(PathBuf),
    /// Get children of a directory (with response channel).
    GetChildren(PathBuf, mpsc::Sender<Vec<FileEntry>>),
}

/// A filesystem model similar to Qt6's QFileSystemModel.
///
/// Features:
/// - Lazy loading of directories
/// - Asynchronous file system operations
/// - Automatic updates via file watching
/// - Caching of file metadata
/// - Icon support
pub struct FileSystemModel {
    root_path: PathBuf,
    cache: Arc<FileSystemCache>,
    watcher: Arc<Mutex<FileSystemWatcher>>,
    task_tx: mpsc::UnboundedSender<FileSystemTask>,
    event_tx: broadcast::Sender<FileSystemEvent>,
    icon_provider: Arc<dyn IconProvider>,
}

impl FileSystemModel {
    /// Create a new filesystem model with the given root path.
    pub fn new(root_path: PathBuf) -> Result<Self, FileSystemError> {
        // Initialize cache
        let cache = Arc::new(FileSystemCache::new());

        // Initialize watcher
        let watcher = Arc::new(Mutex::new(FileSystemWatcher::new()?));

        // Create channels
        let (task_tx, task_rx) = mpsc::unbounded_channel();
        let (event_tx, _) = broadcast::channel(100); // Buffer up to 100 events

        // Initialize icon provider
        let icon_provider: Arc<dyn IconProvider> = Arc::new(MimeIconProvider::new());

        // Spawn async worker task
        let cache_clone = cache.clone();
        let watcher_clone = watcher.clone();
        let event_tx_clone = event_tx.clone();
        tokio::spawn(async move {
            Self::worker_task(task_rx, event_tx_clone, cache_clone, watcher_clone).await;
        });

        let model = Self {
            root_path: root_path.clone(),
            cache,
            watcher,
            task_tx,
            event_tx,
            icon_provider,
        };

        // Start watching root path
        model.watcher.lock().unwrap().watch(&root_path)?;

        Ok(model)
    }

    /// Get children of a directory (lazy loading).
    ///
    /// Returns a future that resolves when the directory is loaded.
    pub async fn get_children(&self, path: &Path) -> Result<Vec<FileEntry>, FileSystemError> {
        // Check cache first
        if let Some(entries) = self.cache.get_children(path) {
            return Ok(entries);
        }

        // Request async load
        let (tx, mut rx) = mpsc::channel(1);
        self.task_tx
            .send(FileSystemTask::GetChildren(path.to_path_buf(), tx))
            .map_err(|_| FileSystemError::ChannelClosed)?;

        rx.recv().await.ok_or(FileSystemError::ChannelClosed)
    }

    /// Refresh a directory (reload from filesystem).
    pub fn refresh(&self, path: &Path) -> Result<(), FileSystemError> {
        println!("FileSystemModel: Refreshing path {:?}", path);
        self.task_tx
            .send(FileSystemTask::LoadDirectory(path.to_path_buf()))
            .map_err(|_| {
                FileSystemError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Worker task died",
                ))
            })
    }

    /// Get file entry for a path.
    pub fn get_entry(&self, path: &Path) -> Option<FileEntry> {
        self.cache.get_entry(path)
    }

    /// Subscribe to filesystem events.
    ///
    /// Returns a receiver that can be used to receive events.
    pub fn subscribe_events(&self) -> broadcast::Receiver<FileSystemEvent> {
        self.event_tx.subscribe()
    }

    /// Get the root path.
    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    /// Load directory entries from filesystem.
    async fn load_directory(path: &Path) -> Result<Vec<FileEntry>, FileSystemError> {
        let mut entries = Vec::new();
        let mut dir = tokio::fs::read_dir(path).await?;

        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            let meta_res = io_uring::stat(&path).await;

            let (file_type, size, modified, created, permissions) = if let Ok(metadata) = meta_res {
                let ft = if metadata.is_dir() {
                    FileType::Directory
                } else if metadata.is_symlink() {
                    FileType::Symlink
                } else if metadata.is_file() {
                    FileType::File
                } else {
                    FileType::Other
                };
                (
                    ft,
                    metadata.len(),
                    metadata.modified()?,
                    metadata.created().ok(),
                    metadata.permissions().mode(),
                )
            } else {
                let metadata = entry.metadata().await?;
                let ft = if metadata.is_dir() {
                    FileType::Directory
                } else if metadata.is_symlink() {
                    FileType::Symlink
                } else if metadata.is_file() {
                    FileType::File
                } else {
                    FileType::Other
                };
                (
                    ft,
                    metadata.len(),
                    metadata.modified()?,
                    metadata.created().ok(),
                    metadata.permissions().mode(),
                )
            };

            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Detect MIME type using MimeDetector
            let mime_type = if file_type == FileType::File {
                crate::filesystem::mime_detector::MimeDetector::detect_mime_type(&path)
            } else {
                None
            };

            let file_metadata = FileMetadata {
                size,
                modified,
                created,
                permissions,
                mime_type,
                is_hidden: name.starts_with('.'),
            };

            entries.push(FileEntry::new(
                path.clone(),
                name,
                file_type,
                file_metadata,
                Some(path.parent().unwrap().to_path_buf()),
            ));
        }

        // Sort entries (directories first, then files, alphabetically)
        entries.sort_by(|a, b| match (a.file_type, b.file_type) {
            (FileType::Directory, FileType::Directory) | (FileType::File, FileType::File) => {
                a.name.cmp(&b.name)
            },
            (FileType::Directory, _) => std::cmp::Ordering::Less,
            (_, FileType::Directory) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });

        Ok(entries)
    }

    /// Async worker task that handles filesystem operations.
    async fn worker_task(
        mut task_rx: mpsc::UnboundedReceiver<FileSystemTask>,
        event_tx: broadcast::Sender<FileSystemEvent>,
        cache: Arc<FileSystemCache>,
        watcher: Arc<Mutex<FileSystemWatcher>>,
    ) {
        loop {
            tokio::select! {
                // Handle tasks
                task = task_rx.recv() => {
                    println!("FileSystemModel: Worker received task {:?}", task);
                    match task {
                        Some(FileSystemTask::LoadDirectory(path)) => {
                            println!("FileSystemModel: Worker loading directory {:?}", path);
                            match Self::load_directory(&path).await {
                                Ok(entries) => {
                                    println!("FileSystemModel: Worker loaded {} entries for {:?}", entries.len(), path);
                                    // Update cache
                                    cache.insert_children(&path, entries.clone());

                                    // Emit event
                                    let _ = event_tx.send(FileSystemEvent::DirectoryLoaded {
                                        path,
                                        entries,
                                    });
                                }
                                Err(e) => {
                                    println!("FileSystemModel: Worker failed to load directory {:?}: {:?}", path, e);
                                    // Error occurred, but we don't emit an error event (just log it)
                                }
                            }
                        }
                        Some(FileSystemTask::RefreshDirectory(path)) => {
                            println!("FileSystemModel: Worker refreshing directory {:?}", path);
                            match Self::load_directory(&path).await {
                                Ok(entries) => {
                                    cache.insert_children(&path, entries.clone());
                                    let _ = event_tx.send(FileSystemEvent::DirectoryLoaded {
                                        path,
                                        entries,
                                    });
                                }
                                Err(e) => {
                                    println!("FileSystemModel: Worker failed to refresh directory {:?}: {:?}", path, e);
                                    // Error occurred, but we don't emit an error event (just log it)
                                }
                            }
                        }
                        Some(FileSystemTask::GetChildren(path, tx)) => {
                            let entries = if let Some(cached) = cache.get_children(&path) {
                                cached
                            } else {
                                match Self::load_directory(&path).await {
                                    Ok(loaded) => {
                                        cache.insert_children(&path, loaded.clone());
                                        let _ = event_tx.send(FileSystemEvent::DirectoryLoaded {
                                            path: path.clone(),
                                            entries: loaded.clone(),
                                        });
                                        loaded
                                    }
                                    Err(_) => Vec::new(),
                                }
                            };
                            let _ = tx.send(entries).await;
                        }
                        None => break, // Channel closed, exit worker
                    }
                }
                // Poll watcher events periodically
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                    let changes = watcher.lock().unwrap().poll_events();
                    for change in changes {
                        match change {
                            FileSystemChange::Created(path) => {
                                // Try to load the new entry
                                if let Ok(entry) = Self::load_entry(&path).await {
                                    cache.insert_entry(entry.clone());
                                    // Also update parent directory
                                    if let Some(parent) = path.parent() {
                                        if let Ok(entries) = Self::load_directory(parent).await {
                                            cache.insert_children(parent, entries.clone());
                                            let _ = event_tx.send(FileSystemEvent::DirectoryLoaded {
                                                path: parent.to_path_buf(),
                                                entries,
                                            });
                                        }
                                    }
                                    let _ = event_tx.send(FileSystemEvent::EntryAdded {
                                        path: path.clone(),
                                        entry,
                                    });
                                }
                            }
                            FileSystemChange::Modified(path) => {
                                // Reload the entry
                                if let Ok(entry) = Self::load_entry(&path).await {
                                    cache.insert_entry(entry.clone());
                                    let _ = event_tx.send(FileSystemEvent::EntryModified {
                                        path: path.clone(),
                                        entry,
                                    });
                                }
                            }
                            FileSystemChange::Removed(path) => {
                                cache.remove_entry(&path);
                                // Also update parent directory
                                if let Some(parent) = path.parent() {
                                    if let Ok(entries) = Self::load_directory(parent).await {
                                        cache.insert_children(parent, entries.clone());
                                        let _ = event_tx.send(FileSystemEvent::DirectoryLoaded {
                                            path: parent.to_path_buf(),
                                            entries,
                                        });
                                    }
                                }
                                let _ = event_tx.send(FileSystemEvent::EntryRemoved {
                                    path,
                                });
                            }
                            FileSystemChange::Renamed { old, new } => {
                                cache.remove_entry(&old);
                                if let Ok(entry) = Self::load_entry(&new).await {
                                    cache.insert_entry(entry.clone());
                                }
                                let _ = event_tx.send(FileSystemEvent::EntryRenamed {
                                    old_path: old,
                                    new_path: new,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    /// Load a single entry from the filesystem.
    async fn load_entry(path: &Path) -> Result<FileEntry, FileSystemError> {
        let meta_res = io_uring::stat(path).await;

        let (file_type, size, modified, created, permissions) = if let Ok(metadata) = meta_res {
            let ft = if metadata.is_dir() {
                FileType::Directory
            } else if metadata.is_symlink() {
                FileType::Symlink
            } else if metadata.is_file() {
                FileType::File
            } else {
                FileType::Other
            };
            (
                ft,
                metadata.len(),
                metadata.modified()?,
                metadata.created().ok(),
                metadata.permissions().mode(),
            )
        } else {
            let metadata = tokio::fs::metadata(path).await?;
            let ft = if metadata.is_dir() {
                FileType::Directory
            } else if metadata.is_symlink() {
                FileType::Symlink
            } else if metadata.is_file() {
                FileType::File
            } else {
                FileType::Other
            };
            (
                ft,
                metadata.len(),
                metadata.modified()?,
                metadata.created().ok(),
                metadata.permissions().mode(),
            )
        };

        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let mime_type = if file_type == FileType::File {
            crate::filesystem::mime_detector::MimeDetector::detect_mime_type(path)
        } else {
            None
        };

        let file_metadata = FileMetadata {
            size,
            modified,
            created,
            permissions,
            mime_type,
            is_hidden: name.starts_with('.'),
        };

        Ok(FileEntry::new(
            path.to_path_buf(),
            name,
            file_type,
            file_metadata,
            path.parent().map(|p| p.to_path_buf()),
        ))
    }
}
