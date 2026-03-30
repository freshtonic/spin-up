pub use spin_lang::*;

/// Re-export the `spin!` proc-macro from `spin-core-macros` so users can
/// write `use spin_up::spin;` and use the macro directly.
pub use spin_core_macros::spin;

pub mod core_net;
