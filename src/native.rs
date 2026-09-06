// Copyright (C) 2026 Luna(a492219408). SPDX-License-Identifier: GPL-3.0-only
use crate::{config::Config, gparam, win};
use std::{
    ffi::c_void,
    fs::{File, OpenOptions},
    io::Write,
    panic::{self, AssertUnwindSafe},
    path::Path,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

static MODULE: AtomicUsize = AtomicUsize::new(0);
static INITIALIZED: OnceLock<bool> = OnceLock::new();
static CONFIG: OnceLock<Config> = OnceLock::new();
static LOG: OnceLock<Mutex<File>> = OnceLock::new();
static COPY: AtomicUsize = AtomicUsize::new(0);
static ACTIVE: AtomicBool = AtomicBool::new(false);
static CALLS: AtomicU64 = AtomicU64::new(0);
static ACCEPTED: AtomicU64 = AtomicU64::new(0);
static REJECTED: AtomicU64 = AtomicU64::new(0);
static PANICS: AtomicU64 = AtomicU64::new(0);
static FOUND: [AtomicU64; 3] = [const { AtomicU64::new(0) }; 3];
static CHANGED: [AtomicU64; 3] = [const { AtomicU64::new(0) }; 3];
static FIRST_REJECT_HEADER: AtomicU64 = AtomicU64::new(0);
static LAST_REJECT: AtomicUsize = AtomicUsize::new(0);

type CopyFn = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> *mut c_void;

pub fn remember_module(module: usize) {
    MODULE.store(module, Ordering::Relaxed);
}

fn log(message: &str) {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    if let Some(file) = LOG.get()
        && let Ok(mut file) = file.lock()
    {
        let _ = writeln!(file, "[{timestamp}] {message}");
        let _ = file.flush();
    }
    win::debug(message);
}

pub fn initialize() -> bool {
    *INITIALIZED.get_or_init(|| match panic::catch_unwind(start) {
        Ok(Ok(())) => true,
        Ok(Err(reason)) => {
            log(&format!("INIT FAILED: {reason}"));
            false
        }
        Err(_) => {
            log("INIT FAILED: Rust panic contained at initializer boundary");
            false
        }
    })
}

fn load_config(path: &Path) -> Result<Config, String> {
    match std::fs::read_to_string(path) {
        Ok(text) if text.len() <= 65536 => Config::parse(&text),
        Ok(_) => Err("configuration exceeds 64 KiB".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            log("configuration absent; defaults disable all three effects");
            Ok(Config::default())
        }
        Err(e) => Err(format!("cannot read {}: {e}", path.display())),
    }
}

fn start() -> Result<(), String> {
    let module = MODULE.load(Ordering::Relaxed);
    if module == 0 {
        return Err("module handle unavailable (load the DLL through ModEngine3)".into());
    }
    let path = win::module_path(module)?;
    let directory = path.parent().ok_or("DLL directory unavailable")?;
    let logfile = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(directory.join("EldenRingPostFX.log"))
        .map_err(|e| format!("cannot create log beside DLL: {e}"))?;
    LOG.set(Mutex::new(logfile))
        .map_err(|_| "logger already initialized")?;
    log(concat!(
        "EldenRingPostFX ",
        env!("CARGO_PKG_VERSION"),
        " | Luna(a492219408) | GPL-3.0-only"
    ));
    log(&format!("DLL={}", path.display()));
    let base = win::main_module()?;
    let game = win::module_path(base)?;
    if !game
        .file_name()
        .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("eldenring.exe"))
    {
        return Err("host executable is not eldenring.exe".into());
    }
    let config = load_config(&directory.join("EldenRingPostFX.ini"))?;
    log(&format!(
        "config={config:?}; changes require a process restart"
    ));
    CONFIG
        .set(config)
        .map_err(|_| "config already initialized")?;
    if !config.observe_only && config.enabled().iter().all(|v| !v) {
        log("DISABLED: all options false; no hook installed");
        return Ok(());
    }
    let image = win::image(base)?;
    let site = image.resolve()?;
    log(&format!(
        "validated import: call_rva=0x{:x}, copy_rva=0x{:x}, function=0x{:x}..0x{:x}; image_size=0x{:x}",
        site.call_rva,
        site.copy_rva,
        site.function.start,
        site.function.end,
        image.headers.image_size
    ));
    drop(image);
    win::pin(crate::elden_ring_postfx_init as *const () as usize)?;
    let relay = win::Relay::near(base + site.call_rva, copy_and_patch as *const () as usize)?;
    COPY.store(base + site.copy_rva, Ordering::Release);
    // Create diagnostics outside DllMain. It never reads or writes game memory.
    std::thread::Builder::new()
        .name("EldenRingPostFX-log".into())
        .spawn(report_changes)
        .map_err(|e| format!("cannot start diagnostics: {e}"))?;
    // SAFETY: The supplied ME3 profile calls this initializer synchronously with
    // load_early=true, before game main. Site and original copy ABI were validated.
    unsafe {
        relay.install(base + site.call_rva, site.original_call)?;
    }
    ACTIVE.store(true, Ordering::Release);
    log("HOOK INSTALLED; waiting for GPARAM imports (this is not confirmation of visual effects)");
    Ok(())
}

unsafe extern "C" fn copy_and_patch(
    destination: *mut c_void,
    source: *const c_void,
    size: usize,
) -> *mut c_void {
    let pointer = COPY.load(Ordering::Acquire);
    // SAFETY: COPY is set to the validated native copy target before publishing the call.
    let original: CopyFn = unsafe { std::mem::transmute(pointer) };
    // SAFETY: Forward the original call's ABI and arguments without changing ownership.
    let result = unsafe { original(destination, source, size) };
    CALLS.fetch_add(1, Ordering::Relaxed);
    let Some(config) = CONFIG.get().copied() else {
        return result;
    };
    let dest = destination as usize;
    let src = source as usize;
    let disjoint = dest
        .checked_add(size)
        .zip(src.checked_add(size))
        .is_some_and(|(de, se)| de <= src || se <= dest);
    if result != destination || dest == 0 || !(0x54..=gparam::MAX_SIZE).contains(&size) || !disjoint
    {
        REJECTED.fetch_add(1, Ordering::Relaxed);
        LAST_REJECT.store(100, Ordering::Relaxed);
        return result;
    }
    let outcome = panic::catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: This exact call copies into a fresh game allocation of `size`
        // bytes. The caller has not published it; the validated allocation path
        // guarantees unique writable ownership during this callback. Source is
        // disjoint. We borrow only until returning and never replace/free the pointer.
        let data = unsafe { std::slice::from_raw_parts_mut(destination.cast::<u8>(), size) };
        match gparam::patch(data, config) {
            Ok(report) => {
                ACCEPTED.fetch_add(1, Ordering::Relaxed);
                for i in 0..3 {
                    FOUND[i].fetch_add(report.found[i] as u64, Ordering::Relaxed);
                    CHANGED[i].fetch_add(report.changed[i] as u64, Ordering::Relaxed);
                }
            }
            Err(error) => {
                REJECTED.fetch_add(1, Ordering::Relaxed);
                LAST_REJECT.store(error as usize + 1, Ordering::Relaxed);
                let mut header = [0; 8];
                header.copy_from_slice(&data[..8]);
                let _ = FIRST_REJECT_HEADER.compare_exchange(
                    0,
                    u64::from_le_bytes(header),
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                );
            }
        }
    }));
    if outcome.is_err() {
        PANICS.fetch_add(1, Ordering::Relaxed);
    }
    result
}

fn snapshot() -> [u64; 10] {
    [
        CALLS.load(Ordering::Relaxed),
        ACCEPTED.load(Ordering::Relaxed),
        REJECTED.load(Ordering::Relaxed),
        PANICS.load(Ordering::Relaxed),
        FOUND[0].load(Ordering::Relaxed),
        FOUND[1].load(Ordering::Relaxed),
        FOUND[2].load(Ordering::Relaxed),
        CHANGED[0].load(Ordering::Relaxed),
        CHANGED[1].load(Ordering::Relaxed),
        CHANGED[2].load(Ordering::Relaxed),
    ]
}

fn report_changes() {
    let mut previous = [u64::MAX; 10];
    let mut elapsed = 0u32;
    loop {
        std::thread::sleep(Duration::from_secs(1));
        elapsed = elapsed.saturating_add(1);
        if !ACTIVE.load(Ordering::Acquire) {
            // An install failure has no need for a permanent reporting thread.
            if elapsed >= 10 {
                return;
            }
            continue;
        }
        let current = snapshot();
        if current != previous {
            log(&format!(
                "imports={} accepted={} rejected={} panics={} rows_found(CA,VIG,SHARP)={:?} rows_selected(CA,VIG,SHARP)={:?} observe_only={} last_reject={} first_reject_header_le=0x{:016x}",
                current[0],
                current[1],
                current[2],
                current[3],
                &current[4..7],
                &current[7..10],
                CONFIG.get().is_some_and(|c| c.observe_only),
                LAST_REJECT.load(Ordering::Relaxed),
                FIRST_REJECT_HEADER.load(Ordering::Relaxed)
            ));
            previous = current;
        }
        if [15, 60].contains(&elapsed) && current[0] == 0 {
            log(
                "no imports observed yet; load a save to test the GPARAM path; installation alone does not prove coverage",
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "C" fn test_copy(
        dest: *mut c_void,
        src: *const c_void,
        size: usize,
    ) -> *mut c_void {
        // SAFETY: Tests supply distinct buffers of the requested size.
        unsafe {
            std::ptr::copy_nonoverlapping(src.cast::<u8>(), dest.cast::<u8>(), size);
        }
        dest
    }

    #[test]
    fn callback_preserves_source_pointer_return_and_invalid_buffers() {
        COPY.store(test_copy as *const () as usize, Ordering::Release);
        CONFIG.set(Config::default()).unwrap();
        let (source, _, _) = gparam::tests::fixture();
        let saved = source.clone();
        let mut expected = source.clone();
        gparam::patch(&mut expected, Config::default()).unwrap();
        let mut dest = vec![0; source.len()];
        let pointer = dest.as_mut_ptr().cast();
        // SAFETY: Distinct, fully initialized source and writable destination buffers.
        let result = unsafe { copy_and_patch(pointer, source.as_ptr().cast(), source.len()) };
        assert_eq!(result, pointer);
        assert_eq!(dest, expected);
        assert_eq!(source, saved);
        let invalid = vec![0xffu8; 256];
        let mut dest = vec![0u8; 256];
        // SAFETY: Same bounds and non-aliasing guarantee for an intentionally invalid payload.
        unsafe {
            copy_and_patch(
                dest.as_mut_ptr().cast(),
                invalid.as_ptr().cast(),
                invalid.len(),
            );
        }
        assert_eq!(dest, invalid);
    }
}
