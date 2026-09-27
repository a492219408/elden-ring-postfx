# Third-party notices and acknowledgements

Original EldenRingPostFX code: Copyright (C) 2026 Luna(a492219408), **GPL-3.0-only**. This notice does not relicense game resources or other third-party works.

## Components present in the binary

The DLL links Rust standard library components. Their licenses, copyright holders and bundled component notices are reproduced in `third_party/Rust-COPYRIGHT-library.html` (source) / `Rust-COPYRIGHT-library.html` (runtime package), copied from the Rust 1.96.0 toolchain used for the release. Preserve that file when redistributing. The project has no third-party Cargo dependencies. Windows system libraries are imported from the operating system and are not bundled.

## References and thanks

- **SoulsFormatsNEXT / SoulsFormats contributors**, including YabberAvocado and Natsu: the public [GPARAM format implementation](https://github.com/soulsmods/SoulsFormatsNEXT/blob/master/SoulsFormats/Formats/GPARAM.cs) informed group/field layout, type numbers and ValueID/time layout. EldenRingPostFX independently implements bounds validation and transformation; it does not link or distribute that library. See the [upstream license](https://github.com/soulsmods/SoulsFormatsNEXT/blob/master/LICENSE).
- **iogus / ioguss**: [No Chromatic Aberration no Vignette no Sharpening](https://www.nexusmods.com/eldenring/mods/8570) supplied the functional reference. Locally available files were compared offline. No resource files from that mod are copied, embedded or distributed, and no endorsement or asset redistribution permission is claimed.
- **ModEngine3 contributors**: [ModEngine3](https://github.com/garyttierney/me3) provides native DLL loading. It is installed separately and is not bundled.
- **Microsoft**: Windows memory and DLL APIs are used according to their published interface. Relevant references include [VirtualAlloc](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-virtualalloc), [VirtualProtect](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-virtualprotect) and [DLL best practices](https://learn.microsoft.com/en-us/windows/win32/dlls/dynamic-link-library-best-practices).

Nuxe, WitchyBND, Oodle, Ghidra and LLVM were used for local research only. They are not distributed and are not required to build or run this mod. Short instruction patterns identify the GPARAM import structure; no complete game functions, shaders or resources are embedded.

Distribute the complete GPL text and matching corresponding source with the arrangements required by the license. The packaging script produces a source archive for the exact commit recorded in the binary package's build manifest.
