
macro_rules! warning {
    ($($arg:tt)*) => {
        eprintln!($($arg)*)
    };
}

pub(crate) use warning;
