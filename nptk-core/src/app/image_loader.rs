//! Async image loading utilities for nptk applications.

use std::path::Path;

/// Async image loading utilities.
pub struct ImageLoader;

impl ImageLoader {
    /// Load an image from file asynchronously.
    pub async fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let image_data = smol::fs::read(path).await?;
        Self::load_from_memory(&image_data).await
    }

    /// Load an image from memory asynchronously.
    pub async fn load_from_memory(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        // Use smol::unblock to run image decoding on a thread pool
        let image_data = data.to_vec();
        let decoded = smol::unblock(move || {
            image::load_from_memory(&image_data)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        }).await?;

        let rgba = decoded.to_rgba8();
        Ok(rgba.into_raw())
    }

    /// Load multiple images from files asynchronously.
    pub async fn load_batch_async<P: AsRef<Path>>(paths: Vec<P>) -> Vec<Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>> {
        let mut results = Vec::new();
        
        for path in paths {
            let result = Self::load_from_file(path).await;
            results.push(result);
        }
        
        results
    }

    /// Load multiple images concurrently.
    pub async fn load_batch_concurrent<P: AsRef<Path> + Send + 'static>(paths: Vec<P>) -> Vec<Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>> {
        use futures::future::join_all;
        
        let futures = paths.into_iter().map(|path| {
            async move {
                Self::load_from_file(path).await
            }
        });
        
        join_all(futures).await
    }
}