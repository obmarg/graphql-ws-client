#[cfg(all(feature = "logging", not(target_arch = "wasm32")))]
macro_rules! trace {
    ($($arg:tt)+) => (
        log::trace!(target: "graphql-ws-client", $($arg)+)
    )
}

#[cfg(any(not(feature = "logging"), target_arch = "wasm32"))]
macro_rules! trace {
    ($($t:tt)*) => {};
}

#[cfg(all(feature = "logging", not(target_arch = "wasm32")))]
#[allow(unused_macros)]
macro_rules! warning {
    ($($arg:tt)+) => (
        log::warn!(target: "graphql-ws-client", $($arg)+)
    )
}

#[cfg(any(not(feature = "logging"), target_arch = "wasm32"))]
#[allow(unused_macros)]
macro_rules! warning {
    ($($t:tt)*) => {};
}

pub(crate) use trace;

#[allow(unused_imports)]
pub(crate) use warning;
