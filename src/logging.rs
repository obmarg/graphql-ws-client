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

pub(crate) use trace;
