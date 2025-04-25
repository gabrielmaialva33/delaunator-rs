//! Utilities for WebAssembly
//!
//! This module provides utility functions for WebAssembly support,
//! specifically for error handling and debugging.
//!
//! When compiling to WebAssembly, it leverages the console_error_panic_hook
//! crate to provide better error messages in the browser console.
//! For non-WASM targets, it provides no-op implementations.

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        extern crate console_error_panic_hook;

        /// Sets up the panic hook in WASM environment
        pub fn set_panic_hook() {
            console_error_panic_hook::set_once();
        }
    } else {
        /// No-op implementation for non-WASM targets
        #[inline]
        pub fn set_panic_hook() {
            // No-op
        }
    }
}

/// Initialize WebAssembly utilities
///
/// This function should be called as early as possible in the WASM initialization
/// to set up error handling and other utilities.
///
/// # Details
///
/// - Sets up a panic hook that will convert Rust panics into JavaScript exceptions
/// - This makes debugging WebAssembly code much easier as errors will appear
///   in the browser console with proper stack traces
///
/// # Examples
///
/// ```no_run
/// // In WebAssembly initialization code
/// delaunator_rs::utils::initialize();
/// ```
#[inline]
pub fn initialize() {
    set_panic_hook();
}
