use std::path::Path;
use std::{env, io};

use uring_file::fs as urfs;
use uring_file::metadata::Metadata;

/// Simple runtime toggle: set `NPTK_USE_IO_URING=0` to disable.
fn enabled() -> bool {
    env::var("NPTK_USE_IO_URING")
        .map(|v| v != "0")
        .unwrap_or(true)
}

/// Get metadata via io_uring.
pub async fn stat(path: &Path) -> io::Result<Metadata> {
    if !enabled() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "io_uring disabled via NPTK_USE_IO_URING",
        ));
    }
    urfs::metadata(path).await
}

/// Read the whole file into bytes via io_uring, fallback to std::fs::read.
pub async fn read(path: &Path) -> io::Result<Vec<u8>> {
    if enabled() {
        if let Ok(bytes) = urfs::read(path).await {
            return Ok(bytes);
        }
    }
    std::fs::read(path)
}

/// Read the whole file into string via io_uring, fallback to std::fs::read_to_string.
pub async fn read_to_string(path: &Path) -> io::Result<String> {
    if enabled() {
        if let Ok(text) = urfs::read_to_string(path).await {
            return Ok(text);
        }
    }
    std::fs::read_to_string(path)
}
