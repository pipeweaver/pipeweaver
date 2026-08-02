// One of sync or async must be enabled for this crate to compile
#[cfg(all(feature = "sync", feature = "async"))]
compile_error!(
    "features `sync` and `async` are mutually exclusive. If you want `sync`, disable default features: \
     pipeweaver-ipc = { features = [\"sync\"], default-features = false }"
);

// But not both at the same time
#[cfg(not(any(feature = "sync", feature = "async")))]
compile_error!(
    "one of `sync` or `async` must be enabled -- there's no sensible default runtime style. \
     e.g. pipeweaver-ipc = { features = [\"ipc\", \"async\", \"ipc-tokio\"], default-features = false }"
);

// Can't use the sync ipc client on the async runtime
#[cfg(all(feature = "ipc", feature = "async", not(feature = "ipc-tokio")))]
compile_error!(
    "the `ipc` feature needs `ipc-tokio` as well when `async` is enabled (interprocess needs its tokio \
     backend to be non-blocking): pipeweaver-ipc = { features = [\"async\", \"ipc\", \"ipc-tokio\"], default-features = false }"
);

// Small macro that basically inverses the above, so we don't attempt to compile anything if we're
// badly configured. This saves seeing cascading errors from modules when the only important error
// is the error thrown from here.
macro_rules! cfg_normal {
    ($($item:item)*) => {
        $(
            #[cfg(not(any(
                all(feature = "sync", feature = "async"),
                not(any(feature = "sync", feature = "async")),
                all(feature = "ipc", feature = "async", not(feature = "ipc-tokio")),
            )))]
            $item
        )*
    };
}

cfg_normal! {
    pub mod client;
    pub mod clients;
    pub mod commands;
    pub use maybe_async;
}
