// --- Feature conflict detection ---
#[cfg(all(feature = "memory-impl", feature = "broker-impl"))]
compile_error!("You cannot enable both `memory-impl` and `broker-impl` features at the same time.");

// --- No backend selected ---
#[cfg(not(any(feature = "memory-impl", feature = "broker-impl")))]
compile_error!("You must enable either the `memory-impl` or `broker-impl` feature.");

// --- Conditional exports ---
#[cfg(feature = "memory-impl")]
pub use memory::*;

#[cfg(feature = "broker-impl")]
pub use broker::*;
