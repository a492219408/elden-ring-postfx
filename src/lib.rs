// Copyright (C) 2026 Luna(a492219408). SPDX-License-Identifier: GPL-3.0-only
pub mod config;
pub mod gparam;
pub mod pe;

#[cfg(all(windows, target_arch = "x86_64"))]
mod native;
#[cfg(all(windows, target_arch = "x86_64"))]
mod win;

#[cfg(all(windows, target_arch = "x86_64"))]
#[unsafe(no_mangle)]
pub extern "C" fn elden_ring_postfx_init() -> bool {
    native::initialize()
}

// Do not allocate, create threads or patch code while holding the loader lock.
#[cfg(all(windows, target_arch = "x86_64"))]
#[unsafe(no_mangle)]
pub extern "system" fn DllMain(module: usize, reason: u32, _reserved: usize) -> i32 {
    if reason == 1 {
        native::remember_module(module);
    }
    1
}
