// Library interface for the honk verification modules.
// This allows the code to be tested independently of the PolkaVM target.
// The modules use no_std-compatible code, but when built as a lib, std
// provides the necessary items (Option, Vec, etc.) through the prelude.

extern crate alloc;

pub mod honk;
pub mod sumcheck;
pub mod vk;
