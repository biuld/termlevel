//! Raw C ABI for the official libghostty-vt 1.0.1 release.
//!
//! Bindings are checked in; consumers need neither Zig nor libclang.
//! All handles, tagged values, borrowed buffers and callbacks follow the
//! contracts in `include/ghostty/vt.h` and its included headers.
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
#![allow(clippy::all)]

include!("bindings.rs");
include!("helpers.rs");

/// Version of the headers and the pinned native artifact.
pub const LIBRARY_VERSION: &str = "1.0.1";
