// SPDX-License-Identifier: LGPL-3.0-only
//! Background task executor for thumbnail generation.
//!
//! This module manages the background generation of thumbnails.
//! It uses a blocking task pool for CPU-bound thumbnail generation and
//! async messaging for coordination.

use crate::filesystem::entry::FileEntry;
use npio::service::io_helpers;
use crate::thumbnail::cache::{ensure_cache_dir, is_thumbnail_fresh, thumbnail_cache_path};
use crate::thumbnail::error::ThumbnailError;
use crate::thumbnail::events::{create_thumbnail_event_channel, ThumbnailEvent};
use std::path::PathBuf;
use tokio::sync::{broadcast, mpsc};

/// Task for thumbnail generation.
#[derive(Debug, Clone)]
struct ThumbnailTask {
    entry: FileEntry,
    size: u32,
}

/// Executor for background thumbnail generation.
pub struct ThumbnailExecutor {
    task_tx: mpsc::UnboundedSender<ThumbnailTask>,
    event_tx: broadcast::Sender<ThumbnailEvent>,
}

impl ThumbnailExecutor {
    /// Create a new thumbnail executor.
    pub fn new() -> Self {
        let (task_tx, task_rx) = mpsc::unbounded_channel();
        let event_tx = create_thumbnail_event_channel();

        let event_tx_clone = event_tx.clone();

        // Spawn background worker
        tokio::spawn(async move {
            Self::worker_task(task_rx, event_tx_clone).await;
        });

        Self { task_tx, event_tx }
    }

    /// Request thumbnail generation for a file entry.
    ///
    /// This queues a background task to generate the thumbnail.
    /// The result will be emitted via the event channel.
    ///
    /// # Arguments
    ///
    /// * `entry` - The file entry to generate a thumbnail for
    /// * `size` - The desired thumbnail size
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the task was queued successfully
    /// * `Err(ThumbnailError)` - If queuing failed
    pub fn request_thumbnail(&self, entry: FileEntry, size: u32) -> Result<(), ThumbnailError> {
        let task = ThumbnailTask { entry, size };

        self.task_tx.send(task).map_err(|e| {
            ThumbnailError::Unknown(format!("Failed to queue thumbnail task: {}", e))
        })?;

        Ok(())
    }

    /// Subscribe to thumbnail events.
    ///
    /// Returns a receiver that will receive events when thumbnails are ready or fail.
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<ThumbnailEvent> {
        self.event_tx.subscribe()
    }

    /// Background worker task that processes thumbnail generation requests.
    async fn worker_task(
        mut task_rx: mpsc::UnboundedReceiver<ThumbnailTask>,
        event_tx: broadcast::Sender<ThumbnailEvent>,
    ) {
        while let Some(task) = task_rx.recv().await {
            let entry_path = task.entry.path.clone();
            let size = task.size;

            // Generate thumbnail in async context (since cache checks are now async)
            let result = Self::generate_thumbnail_async(&task.entry, task.size).await;

            match result {
                Ok(thumbnail_path) => {
                    let event = ThumbnailEvent::ThumbnailReady {
                        entry_path,
                        thumbnail_path,
                        size,
                    };
                    let _ = event_tx.send(event);
                },
                Err(e) => {
                    let event = ThumbnailEvent::ThumbnailFailed {
                        entry_path,
                        error: e.to_string(),
                        size,
                    };
                    let _ = event_tx.send(event);
                },
            }
        }
    }

    /// Generate a thumbnail for a file entry (Async version).
    ///
    /// This orchestrates the generation process, calling async cache checks
    /// and offloading blocking generation work.
    async fn generate_thumbnail_async(entry: &FileEntry, size: u32) -> Result<PathBuf, ThumbnailError> {
        // Ensure cache directory exists (now async)
        ensure_cache_dir(size).await.map_err(|e| ThumbnailError::CacheError(e.to_string()))?;

        // Get cache path
        let thumbnail_path = thumbnail_cache_path(entry, size);

        // Check if thumbnail already exists and is fresh (now async)
        // Note: thumbnail_path.exists() is synchronous, but we can use tokio::fs::metadata
        // inside is_thumbnail_fresh which is fully async.
        // We can skip the separate exists check as is_thumbnail_fresh handles it.
        if is_thumbnail_fresh(&thumbnail_path, &entry.path).await {
            log::debug!("Thumbnail cache hit: {:?}", thumbnail_path);
            return Ok(thumbnail_path);
        }

        log::debug!("Generating thumbnail for {:?} at size {}", entry.path, size);

        // Generate thumbnail using thumbnailify
        // thumbnailify operations are blocking, so we offload them to spawn_blocking
        let entry_path = entry.path.clone();
        let thumbnail_path_clone = thumbnail_path.clone();
        
        let generated_path = tokio::task::spawn_blocking(move || {
            // thumbnailify uses ThumbnailSize enum, map our size to it
            let thumbnail_size = if size <= 128 {
                thumbnailify::ThumbnailSize::Normal
            } else {
                thumbnailify::ThumbnailSize::Large
            };

            thumbnailify::generate_thumbnail(&entry_path, thumbnail_size)
                .map_err(|e| {
                    ThumbnailError::GenerationFailed(format!("thumbnailify error: {:?}", e))
                })
        })
        .await
        .map_err(|e| ThumbnailError::Unknown(format!("Task execution error: {}", e)))??;

        // Copy generated thumbnail to our cache location
        // Use async file copy helper
        io_helpers::copy_file(&generated_path, &thumbnail_path)
            .await
            .map_err(|e| {
                ThumbnailError::CacheError(format!("Failed to copy thumbnail to cache: {}", e))
            })?;

        log::info!("Thumbnail generated and cached: {:?}", thumbnail_path);

        Ok(thumbnail_path)
    }
}

impl Default for ThumbnailExecutor {
    fn default() -> Self {
        Self::new()
    }
}
