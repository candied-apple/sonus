use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::OnceLock;

const MAX_LOG_ENTRIES: usize = 1000;

struct LogBuffer {
    entries: VecDeque<String>,
}

static LOGGER: OnceLock<Mutex<LogBuffer>> = OnceLock::new();

fn logger() -> &'static Mutex<LogBuffer> {
    LOGGER.get_or_init(|| Mutex::new(LogBuffer { entries: VecDeque::with_capacity(MAX_LOG_ENTRIES) }))
}

pub fn push(msg: String) {
    if let Ok(mut guard) = logger().lock() {
        if guard.entries.len() >= MAX_LOG_ENTRIES {
            guard.entries.pop_front();
        }
        guard.entries.push_back(msg);
    }
}

pub fn entries() -> Vec<String> {
    if let Ok(guard) = logger().lock() {
        guard.entries.iter().cloned().collect()
    } else {
        vec![]
    }
}

pub fn clear() {
    if let Ok(mut guard) = logger().lock() {
        guard.entries.clear();
    }
}

/// Log a formatted message into the live log buffer.
/// This is a replacement for eprintln! — use it everywhere
/// background diagnostics should be captured.
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        $crate::log::push(format!($($arg)*));
    };
}
