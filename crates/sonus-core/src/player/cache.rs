use std::path::PathBuf;
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

static CACHE_SIZE_ESTIMATE: AtomicU64 = AtomicU64::new(u64::MAX);

pub fn get_cache_dir() -> PathBuf {
    let base = dirs::cache_dir().unwrap_or_else(|| PathBuf::from(".cache"));
    let path = base.join("sonus");
    let _ = fs::create_dir_all(&path);
    path
}

pub fn add_file_size(size: u64) {
    CACHE_SIZE_ESTIMATE.fetch_add(size, Ordering::Relaxed);
}

pub fn warm_cache_estimate() {
    let mut current_size = 0u64;
    if let Ok(entries) = std::fs::read_dir(get_cache_dir()) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                if let Ok(meta) = entry.metadata() {
                    current_size += meta.len();
                }
            }
        }
    }
    CACHE_SIZE_ESTIMATE.store(current_size, Ordering::Relaxed);
}

pub fn prune_cache(max_size_bytes: u64) -> std::io::Result<u64> {
    let cache_dir = get_cache_dir();

    if CACHE_SIZE_ESTIMATE.load(Ordering::Relaxed) <= max_size_bytes {
        return Ok(0);
    }

    let entries = fs::read_dir(&cache_dir)?;

    let mut files = Vec::new();
    let mut current_size = 0u64;

    for entry in entries {
        if let Ok(entry) = entry {
            if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                if let Ok(metadata) = entry.metadata() {
                    let path = entry.path();
                    let size = metadata.len();
                    let modified = metadata.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    current_size += size;
                    files.push((path, size, modified));
                }
            }
        }
    }

    CACHE_SIZE_ESTIMATE.store(current_size, Ordering::Relaxed);

    if current_size <= max_size_bytes {
        return Ok(0);
    }

    files.sort_by_key(|f| f.2);

    let mut bytes_freed = 0u64;
    for (path, size, _) in files {
        if current_size <= max_size_bytes {
            break;
        }
        if let Ok(()) = fs::remove_file(&path) {
            current_size -= size;
            bytes_freed += size;
        }
    }

    CACHE_SIZE_ESTIMATE.store(current_size, Ordering::Relaxed);

    Ok(bytes_freed)
}
