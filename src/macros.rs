#[macro_export]
macro_rules! fatal {
    ($($t:tt)*) => {{
        log::error!($($t)*);
        std::process::exit(1);
    }};
}
