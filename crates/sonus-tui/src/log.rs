#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        sonus_core::log::push(format!("{}", format_args!($($arg)*)));
    };
}



