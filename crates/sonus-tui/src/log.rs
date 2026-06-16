#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        sonus_core::log::push(format!("{}", format_args!($($arg)*)));
    };
}

#[allow(dead_code)]
pub fn entries() -> Vec<String> {
    sonus_core::log::entries()
}

#[allow(dead_code)]
pub fn clear() {
    sonus_core::log::clear();
}
