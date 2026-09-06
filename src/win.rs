// Copyright (C) 2026 Luna(a492219408). SPDX-License-Identifier: GPL-3.0-only
use crate::pe::{Headers, Image, relevant};
use std::{
    ffi::{OsString, c_void},
    mem,
    os::windows::ffi::OsStringExt,
    path::PathBuf,
    ptr,
};

#[repr(C)]
#[derive(Default)]
struct MemoryInfo {
    base: usize,
    allocation_base: usize,
    allocation_protect: u32,
    partition_id: u16,
    region_size: usize,
    state: u32,
    protect: u32,
    kind: u32,
}
#[repr(C)]
#[derive(Default)]
struct SystemInfo {
    architecture: u32,
    page_size: u32,
    minimum: usize,
    maximum: usize,
    mask: usize,
    processors: u32,
    processor_type: u32,
    granularity: u32,
    level: u16,
    revision: u16,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(name: *const u16) -> usize;
    fn GetModuleHandleExW(flags: u32, address: *const u16, module: *mut usize) -> i32;
    fn GetModuleFileNameW(module: usize, buffer: *mut u16, size: u32) -> u32;
    fn GetCurrentProcess() -> usize;
    fn GetSystemInfo(info: *mut SystemInfo);
    fn ReadProcessMemory(
        process: usize,
        address: *const c_void,
        output: *mut c_void,
        size: usize,
        read: *mut usize,
    ) -> i32;
    fn VirtualQuery(address: *const c_void, info: *mut MemoryInfo, size: usize) -> usize;
    fn VirtualAlloc(address: *mut c_void, size: usize, kind: u32, protection: u32) -> *mut c_void;
    fn VirtualFree(address: *mut c_void, size: usize, kind: u32) -> i32;
    fn VirtualProtect(
        address: *mut c_void,
        size: usize,
        protection: u32,
        previous: *mut u32,
    ) -> i32;
    fn FlushInstructionCache(process: usize, address: *const c_void, size: usize) -> i32;
    fn OutputDebugStringW(text: *const u16);
}

fn os_error(context: &str) -> String {
    format!("{context}: {}", std::io::Error::last_os_error())
}

pub fn debug(message: &str) {
    let text: Vec<u16> = format!("EldenRingPostFX: {message}\0")
        .encode_utf16()
        .collect();
    // SAFETY: NUL-terminated UTF-16 storage lives through the call.
    unsafe {
        OutputDebugStringW(text.as_ptr());
    }
}

pub fn module_path(module: usize) -> Result<PathBuf, String> {
    let mut capacity = 512;
    loop {
        let mut buffer = vec![0; capacity];
        // SAFETY: Buffer is writable for the supplied length; handles originate in the loader.
        let n =
            unsafe { GetModuleFileNameW(module, buffer.as_mut_ptr(), capacity as u32) } as usize;
        if n == 0 {
            return Err(os_error("GetModuleFileNameW"));
        }
        if n < capacity {
            return Ok(PathBuf::from(OsString::from_wide(&buffer[..n])));
        }
        capacity *= 2;
        if capacity > 32768 {
            return Err("module path too long".into());
        }
    }
}

pub fn main_module() -> Result<usize, String> {
    // SAFETY: NULL requests the current process image without changing its reference count.
    let module = unsafe { GetModuleHandleW(ptr::null()) };
    if module == 0 {
        Err(os_error("GetModuleHandleW"))
    } else {
        Ok(module)
    }
}

pub fn pin(address: usize) -> Result<(), String> {
    let mut module = 0;
    // SAFETY: FROM_ADDRESS interprets the argument as a function address in this DLL.
    if unsafe { GetModuleHandleExW(1 | 4, address as *const u16, &mut module) } == 0 {
        Err(os_error("pin DLL"))
    } else {
        Ok(())
    }
}

pub fn read(address: usize, size: usize) -> Result<Vec<u8>, String> {
    if size == 0 || size > 512 * 1024 * 1024 || address.checked_add(size).is_none() {
        return Err("invalid memory snapshot range".into());
    }
    let mut output = vec![0; size];
    let mut copied = 0;
    // SAFETY: Windows validates source readability; output owns the requested capacity.
    let ok = unsafe {
        ReadProcessMemory(
            GetCurrentProcess(),
            address as *const c_void,
            output.as_mut_ptr().cast(),
            size,
            &mut copied,
        )
    };
    if ok == 0 || copied != size {
        Err(os_error("ReadProcessMemory"))
    } else {
        Ok(output)
    }
}

pub fn image(base: usize) -> Result<Image, String> {
    let headers = Headers::parse(&read(base, 4096)?)?;
    let mut sections = Vec::new();
    for s in headers.sections.iter().filter(|s| relevant(s)) {
        let size = s.virtual_size.min(s.raw_size);
        sections.push((
            s.clone(),
            read(base.checked_add(s.rva).ok_or("base overflow")?, size)?,
        ));
    }
    Ok(Image { headers, sections })
}

fn system_info() -> SystemInfo {
    let mut info = SystemInfo::default();
    // SAFETY: SYSTEM_INFO layout matches the Windows x64 ABI.
    unsafe {
        GetSystemInfo(&mut info);
    }
    info
}

fn protect(address: usize, size: usize, protection: u32) -> Result<u32, String> {
    let mut old = 0;
    // SAFETY: Windows validates the mapped range; callers own or exclusively patch it.
    if unsafe { VirtualProtect(address as *mut c_void, size, protection, &mut old) } == 0 {
        Err(os_error("VirtualProtect"))
    } else {
        Ok(old)
    }
}

fn flush(address: usize, size: usize) -> Result<(), String> {
    // SAFETY: The address range belongs to this process. No pointer is dereferenced by Rust.
    if unsafe { FlushInstructionCache(GetCurrentProcess(), address as *const c_void, size) } == 0 {
        Err(os_error("FlushInstructionCache"))
    } else {
        Ok(())
    }
}

/// Relay is allocated near the call and jumps absolutely to the Rust callback.
/// It remains mapped for the process lifetime after installation.
pub struct Relay {
    address: usize,
}
impl Drop for Relay {
    fn drop(&mut self) {
        // SAFETY: Unpublished relay exclusively owns this VirtualAlloc allocation.
        unsafe {
            VirtualFree(self.address as *mut c_void, 0, 0x8000);
        }
    }
}

impl Relay {
    pub fn near(call: usize, callback: usize) -> Result<Self, String> {
        let info = system_info();
        let granularity = info.granularity as usize;
        if !granularity.is_power_of_two() || info.page_size == 0 {
            return Err("invalid allocation granularity".into());
        }
        let lo = call
            .saturating_add(5)
            .saturating_sub(i32::MAX as usize)
            .max(info.minimum);
        let hi = call
            .saturating_add(5)
            .saturating_add(i32::MAX as usize)
            .min(info.maximum);
        let mut cursor = lo;
        while cursor < hi {
            let mut region = MemoryInfo::default();
            // SAFETY: VirtualQuery reports metadata without dereferencing the supplied address.
            if unsafe {
                VirtualQuery(
                    cursor as *const c_void,
                    &mut region,
                    mem::size_of::<MemoryInfo>(),
                )
            } == 0
            {
                break;
            }
            let end = region
                .base
                .checked_add(region.region_size)
                .ok_or("memory region overflow")?;
            if end <= cursor {
                break;
            }
            if region.state == 0x10000 {
                let candidate = cursor.saturating_add(granularity - 1) & !(granularity - 1);
                if candidate + info.page_size as usize <= end.min(hi) {
                    // SAFETY: Request a new, disjoint RW allocation; never commit an existing mapping.
                    let pointer = unsafe {
                        VirtualAlloc(
                            candidate as *mut c_void,
                            info.page_size as usize,
                            0x3000,
                            0x04,
                        )
                    };
                    if !pointer.is_null() {
                        let relay = Self {
                            address: pointer as usize,
                        };
                        let mut code = [0u8; 14];
                        code[..6].copy_from_slice(&[0xff, 0x25, 0, 0, 0, 0]);
                        code[6..].copy_from_slice(&callback.to_le_bytes());
                        // SAFETY: New private RW allocation has at least a page of capacity.
                        unsafe {
                            ptr::copy_nonoverlapping(code.as_ptr(), pointer.cast(), code.len());
                        }
                        protect(relay.address, 14, 0x20)?;
                        flush(relay.address, 14)?;
                        return Ok(relay);
                    }
                }
            }
            cursor = end;
        }
        Err("cannot allocate a relay within rel32 reach".into())
    }

    /// Must run synchronously before game main, without another writer/executor at
    /// this call site. No suspension of game threads or late injection is supported.
    pub unsafe fn install(self, call: usize, expected: [u8; 5]) -> Result<(), String> {
        let page = system_info().page_size as usize;
        if page == 0 || call % page + 5 > page {
            return Err("call crosses a protection page".into());
        }
        if read(call, 5)? != expected || expected[0] != 0xe8 {
            return Err("call changed before installation".into());
        }
        let delta = (self.address as i128) - (call as i128 + 5);
        let delta = i32::try_from(delta).map_err(|_| "relay out of reach")?;
        let mut replacement = [0xe8, 0, 0, 0, 0];
        replacement[1..].copy_from_slice(&delta.to_le_bytes());
        let old = protect(call, 5, 0x40)?;
        // Keep mapped even if protection restoration/rollback fails after publication.
        mem::forget(self);
        // SAFETY: Caller guarantees exclusive early installation; VirtualProtect succeeded.
        unsafe {
            ptr::copy_nonoverlapping(replacement.as_ptr(), call as *mut u8, 5);
        }
        let finish = protect(call, 5, old).and_then(|_| flush(call, 5));
        if let Err(reason) = finish {
            let rollback = (|| {
                protect(call, 5, 0x40)?;
                if read(call, 5)? != replacement {
                    return Err("call ownership lost during rollback".to_owned());
                }
                // SAFETY: Still exclusively owned; protection was made writable above.
                unsafe {
                    ptr::copy_nonoverlapping(expected.as_ptr(), call as *mut u8, 5);
                }
                protect(call, 5, old)?;
                flush(call, 5)
            })();
            return Err(format!(
                "hook publication failed: {reason}; rollback={rollback:?}"
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern "C" fn original(value: usize) -> usize {
        value + 3
    }
    extern "C" fn replacement(value: usize) -> usize {
        value + 17
    }

    #[test]
    fn executes_real_call_relay_and_preserves_return_and_stack() {
        // Build an owned test function with a real Windows ABI CALL. Never touches game memory.
        // SAFETY: Reserve a new private page for the synthetic test function.
        let p = unsafe { VirtualAlloc(ptr::null_mut(), 4096, 0x3000, 0x04) } as usize;
        assert_ne!(p, 0);
        let owner = Relay { address: p };
        let mut code = [0xcc; 64];
        code[..4].copy_from_slice(&[0x48, 0x83, 0xec, 0x28]); // shadow space + stack alignment
        code[4..9].copy_from_slice(&[0xe8, 23, 0, 0, 0]); // call internal tail relay at +32
        code[9..14].copy_from_slice(&[0x48, 0x83, 0xc4, 0x28, 0xc3]);
        code[32..38].copy_from_slice(&[0xff, 0x25, 0, 0, 0, 0]);
        code[38..46].copy_from_slice(&(original as *const () as usize).to_le_bytes());
        // SAFETY: Owned RW page is large enough for code.
        unsafe {
            ptr::copy_nonoverlapping(code.as_ptr(), p as *mut u8, code.len());
        }
        protect(p, 4096, 0x20).unwrap();
        flush(p, 64).unwrap();
        // SAFETY: Synthetic code follows the Windows x64 ABI for this signature.
        let function: extern "C" fn(usize) -> usize = unsafe { mem::transmute(p) };
        assert_eq!(function(20), 23);
        let relay = Relay::near(p + 4, replacement as *const () as usize).unwrap();
        // SAFETY: Single-threaded test has exclusive ownership; function is not executing.
        unsafe { relay.install(p + 4, [0xe8, 23, 0, 0, 0]) }.unwrap();
        assert_eq!(function(20), 37);
        assert_eq!(function(100), 117);
        let wrong = Relay::near(p + 4, original as *const () as usize).unwrap();
        // SAFETY: Same exclusive ownership; deliberately wrong expected bytes must be rejected.
        assert!(unsafe { wrong.install(p + 4, [0xe8, 23, 0, 0, 0]) }.is_err());
        assert_eq!(function(20), 37);
        drop(owner);
    }
}
