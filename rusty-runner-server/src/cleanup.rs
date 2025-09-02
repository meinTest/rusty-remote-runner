//! Cleanup code copied for `log-server` crate

use std::time::{Duration, SystemTime};

use crate::process::working_directory;

/// Starts periodic cleanup
///
/// This spawns a background task for each endpoint, running every 8 hours.
/// Runs [`cleanup_endpoint`].
#[allow(rustdoc::private_intra_doc_links)] // don't care, mostly for in IDE docs anyway
pub fn start_cleanup_task(max_age: Option<Duration>, max_size: Option<usize>) {
    tokio::spawn(async move {
        // Cleanup runs every 8 hours. This is a tradeoff between resource usage and timely cleanup.
        let mut interval = tokio::time::interval(Duration::from_secs(8 * 60 * 60));
        loop {
            interval.tick().await;
            if let Err(e) = cleanup_endpoint(max_age, max_size).await {
                log::warn!(e:debug; "cleanup failed");
            }
        }
    });
}

/// Cleans up a single directory according
///
/// - First removes directories older than `max_age`.
/// - Then removes oldest directories until under limit of `max_size` in bytes.
///
/// Age is determined by [`std::fs::Metadata::modified`].
async fn cleanup_endpoint(
    max_age: Option<Duration>,
    max_size: Option<usize>,
) -> std::io::Result<()> {
    let dir = working_directory();

    // Remove directories older than max_age
    if let Some(max_age) = max_age {
        let mut rd = tokio::fs::read_dir(&dir).await?;
        let now = SystemTime::now();
        while let Some(entry) = rd.next_entry().await? {
            let modified = entry
                .metadata()
                .await?
                .modified()
                .unwrap_or(SystemTime::UNIX_EPOCH);
            if now.duration_since(modified).unwrap_or(Duration::ZERO) > max_age {
                let path = entry.path();
                let file_type = entry.file_type().await?;
                if file_type.is_dir() {
                    tokio::fs::remove_dir_all(&path).await?;
                    log::trace!(path:debug; "deleted old artifact directory due to max_age");
                } else if file_type.is_file() {
                    tokio::fs::remove_file(&path).await?;
                    log::trace!(path:debug; "deleted old artifact file due to max_age");
                }
            }
        }
    }

    // Remove oldest directories if total size exceeds max_size
    if let Some(max_size) = max_size {
        // Collect (path, modified_time, size) for sorting
        let mut entries = Vec::new();
        let mut rd = tokio::fs::read_dir(&dir).await?;
        while let Some(entry) = rd.next_entry().await? {
            let modified = entry
                .metadata()
                .await?
                .modified()
                .unwrap_or(SystemTime::UNIX_EPOCH);
            entries.push((entry, modified));
        }
        // Sort by modified time descending (newest first)
        entries.sort_by_key(|(_, modified)| std::cmp::Reverse(*modified));

        // Iterate newest to oldest, keep until max_size is exceeded, then delete the rest
        let mut total_size_seen = 0;
        for (entry, _ts) in entries {
            // No need to calculate size once the max has been reached
            if total_size_seen <= max_size {
                let dir_size = dir_size(entry.path()).await;
                let path = entry.path();
                log::trace!(path:debug, dir_size; "found entry");
                total_size_seen += dir_size;
            }
            // Remove if the total with this is now above, or already was.
            if total_size_seen > max_size {
                let path = entry.path();
                let file_type = entry.file_type().await?;
                if file_type.is_dir() {
                    tokio::fs::remove_dir_all(&path).await?;
                    log::trace!(path:debug; "deleted old artifact directory due to max_size");
                } else if file_type.is_file() {
                    tokio::fs::remove_file(&path).await?;
                    log::trace!(path:debug; "deleted old artifact file due to max_size");
                }
            }
        }
    }
    Ok(())
}

/// Recursively calculates the total size of all files in a directory or a single file.
///
/// If the path is a file, returns its size. If it's a directory, sums all contained files/directories recursively.
/// This does a depth-first, sequential search, which is not very fast but intentionally won't stress the system.
#[allow(clippy::cast_possible_truncation)]
fn dir_size(
    path: std::path::PathBuf,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = usize> + Send>> {
    Box::pin(async move {
        // Check if path is a file or directory
        let Ok(meta) = tokio::fs::metadata(&path).await else {
            log::warn!("failed to read metadata, defaulting to zero");
            return 0;
        };
        if meta.is_file() {
            meta.len() as usize
        } else if meta.is_dir() {
            // Directory case
            let Ok(mut rd) = tokio::fs::read_dir(&path).await else {
                log::warn!("failed to read dir, defaulting to zero");
                return 0;
            };
            let mut size = 0;
            while let Some(entry) = rd.next_entry().await.unwrap_or(None) {
                size += dir_size(entry.path()).await;
            }
            size
        } else {
            log::warn!("found non-dir non-file, shouldn't exist in artifact store");
            0
        }
    })
}
