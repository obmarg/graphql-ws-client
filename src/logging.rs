#[cfg(not(feature = "no-logging"))]
macro_rules! trace {
    ($($arg:tt)+) => (
        log::trace!(target: "graphql-ws-client", $($arg)+)
    )
}

#[cfg(feature = "no-logging")]
macro_rules! trace {
    ($($t:tt)*) => {};
}

#[cfg(not(feature = "no-logging"))]
macro_rules! warning {
    ($($arg:tt)+) => (
        log::warn!(target: "graphql-ws-client", $($arg)+)
    )
}

#[cfg(feature = "no-logging")]
macro_rules! warning {
    ($($t:tt)*) => {};
}

pub(crate) use trace;
pub(crate) use warning;
