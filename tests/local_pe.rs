// Copyright (C) 2026 Luna(a492219408). SPDX-License-Identifier: GPL-3.0-only
use elden_ring_postfx::pe::Image;

/// Optional validation against an executable supplied by the developer. No game
/// files, hashes, dumps or generated fixtures are included in the source/build.
#[test]
#[ignore = "set POSTFX_TEST_EXE to your own eldenring.exe; no game is launched"]
fn supplied_executable_resolves_and_modified_code_is_rejected() {
    let path = std::env::var_os("POSTFX_TEST_EXE").expect("POSTFX_TEST_EXE");
    let file = std::fs::read(path).unwrap();
    let mut image = Image::from_file(&file).unwrap();
    let site = image.resolve().unwrap();
    println!("{site:?}");
    // Mutation of the allocation path must prevent installation, even if the
    // main signature still matches. Restore before checking signature ambiguity.
    let (section, bytes) = image
        .sections
        .iter_mut()
        .find(|(s, d)| site.call_rva >= s.rva && site.call_rva < s.rva + d.len())
        .unwrap();
    let offset = site.call_rva - section.rva;
    let saved = bytes[offset - 0x7d];
    bytes[offset - 0x7d] ^= 1;
    assert!(image.resolve().unwrap_err().contains("allocation"));
    let (_, bytes) = image
        .sections
        .iter_mut()
        .find(|(s, d)| site.call_rva >= s.rva && site.call_rva < s.rva + d.len())
        .unwrap();
    bytes[offset - 0x7d] = saved;
    let duplicate = bytes[offset - 9..offset + 51].to_vec();
    bytes[0..duplicate.len()].copy_from_slice(&duplicate);
    assert!(image.resolve().unwrap_err().contains("found 2"));
}
