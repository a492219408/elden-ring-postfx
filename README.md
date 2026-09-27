# Elden Ring Native PostFX

[简体中文](README.zh-CN.md)

A configurable **ModEngine3 Native DLL** that removes Elden Ring's chromatic aberration, vignette and built-in sharpening. Each effect has an independent switch. No replacement game resources or extra shaders are needed.

**Version 1.0.0 · Windows x64 · GPL-3.0-only · Luna(a492219408)**

The effect and hook implementation is unchanged from the tested `0.1.0-preview-2b1f244`. This first stable release adds release packaging, documentation and license notices. The maintainer tested the preview on **App Ver. 1.17.1 / Regulation Ver. 1.17.1** and reported no issues; the supplied log confirms all three overrides were exercised without parser rejections or caught panics. This does not establish coverage of every area or mod combination.

## Install

Requires Elden Ring, Windows x64 and [ModEngine3](https://github.com/garyttierney/me3) with early native loading and named initializer support.

1. Extract `EldenRingPostFX-1.0.0.zip` into a writable folder and keep its structure.
2. Disable all variants of the old **No Chromatic Aberration no Vignette no Sharpening** resource mod if installed.
3. Open the included `EldenRingPostFX.me3` with ModEngine3 and launch the game. The supplied profile uses offline mode.
4. All three effects are disabled by default. Edit `natives/EldenRingPostFX.ini` to choose your settings, then fully exit and restart the game.

No game files are changed on disk. Nuxe, WitchyBND, Oodle and Ghidra are development research tools, not installation or build requirements. The DLL uses the static CRT and does not require an additional Visual C++ runtime package.

```ini
[postfx]
disable_chromatic_aberration = true
disable_vignette = true
disable_sharpening = true
observe_only = false
```

`true` removes the corresponding effect; `false` leaves its incoming value unchanged. It cannot restore values already removed by another mod. All three switches can be combined freely. With all three off and observation disabled, no hook is installed. A missing INI uses the defaults; invalid values, unknown keys or duplicate keys reject initialization.

`observe_only = true` counts the records that would be changed without modifying them. It still installs the import hook and is intended for diagnosis.

## Use with an existing profile

Keep the DLL and INI together. Add this entry to your `.me3` profile, adjusting the path relative to that profile:

```toml
[[natives]]
path = 'mods/eldenring/EldenRingPostFX/natives/EldenRingPostFX.dll'
enabled = true
optional = false
load_early = true
initializer = { function = "elden_ring_postfx_init" }
```

Keep `load_early` and the named initializer exactly as shown. Disable the old resource packages for all five variants, including the single-effect variants. Otherwise they may still override newer game data and keep effects disabled when the DLL setting is off.

To uninstall, exit the game and remove or disable this native entry. Hot loading, reloading configuration in-game and unloading the DLL in a running process are not supported.

## Compatibility and scope

| Game / regulation | Evidence |
| --- | --- |
| 1.17.1 / 1.17.1 | Maintainer gameplay report and runtime log from the preview; executable and resource regression checks |
| 1.17 / 1.17 | Maintainer fixed-viewpoint comparisons of all three effects; executable and resource checks |
| 1.16.2 / 1.16.1 | Offline executable and resource checks; no DLL gameplay validation |
| Future versions | May work when the validated code and resource structure is preserved; not guaranteed |

Samples came from installations with Shadow of the Erdtree. Gameplay coverage of every DLC area, every weather/time state, the base-game-only installation and other mod combinations has not been established. The preview screenshots come from 1.17, not 1.17.1. No performance improvement is claimed.

The DLL changes the same three GPARAM properties targeted by the reference resource mod. It does not disable sharpening supplied by ReShade, graphics drivers or unrelated filters. Parameter mods are processed as loaded: selected target values become zero, while other fields retain their incoming values. Mods that patch the same import function can conflict.

## How it works

During GPARAM import, the game allocates and copies a new buffer. The DLL redirects one validated copy call, invokes the original function first, then applies the selected changes to that new buffer. Source data, allocation, size and return value are preserved. There is no global memcpy hook and no per-frame memory scan.

| Group | Full field name | Override |
| --- | --- | --- |
| `Tone Map[Yebis]` | `GS Presents - Chromatic Aberration Enable` | Integer zero, type 3 or 7 |
| `Tone Map[Yebis]` | `GS Presents - Vignetting Power` | Float zero, type 9 |
| `Anti-Alias-Editor` | `AntiAlias - SharpenIntensity` | Float zero, type 9 |

All ValueID/time rows of a matching field are handled. GPARAM v5 bounds, types and overlapping intervals are checked before any write; rejected buffers remain unchanged. Executable resolution validates a unique instruction pattern, function boundaries, allocation path, strings and copy function structure. It does not rely on a fixed game address or a version hash allowlist. Structural checks reduce incorrect matches but do not guarantee compatibility with unknown updates.

The old full resource packs contain an older `m61_00_0000` file. This DLL retains the newer game's unrelated changes there instead of replacing it with old data.

## Logs

`natives/EldenRingPostFX.log` is replaced at each initialization. Copy it before restarting when reporting a problem. Include game/regulation versions, the INI and other enabled mods.

- `INIT FAILED`: a configuration, process, signature or installation check failed. A successful game launch alone does not mean the mod is active.
- `HOOK INSTALLED`: the hook is ready; actual imports are recorded after loading a save.
- `imports / accepted / rejected / panics`: calls seen, supported GPARAM buffers, rejected buffers and caught Rust panics.
- `rows_found(CA,VIG,SHARP)`: recognized rows. `rows_selected`: nonzero rows selected for removal; observation mode counts without writing. Reloads accumulate, so these are not unique asset counts.
- `last_reject`: 1 = header/size, 2 = version, 3 = bounds, 4 = layout, 5 = complexity, 6 = type, 7 = overlap, 100 = copy-buffer conditions. `first_reject_header_le` stores only the first rejected buffer's eight-byte header.

## Build and package

Build on Windows using Rust 1.96 or newer, the `x86_64-pc-windows-msvc` target and Visual Studio C++ build tools. There are no third-party Rust crates and no private file or game sample dependencies.

```powershell
mise exec -- pwsh -NoProfile -File scripts/Build.ps1
```

The script runs formatting, Clippy, tests and the release build. Without mise, run the script with PowerShell directly. The DLL is `target/x86_64-pc-windows-msvc/release/elden_ring_postfx.dll`. The source ZIP can be built without Git or the private repository.

To package a release, use a clean Git checkout, Git, PowerShell 7 and Python 3.11+:

```powershell
mise exec -- pwsh -NoProfile -File scripts/Package.ps1
```

Packaging rebuilds the DLL, checks the public file allowlist and reachable Git history, verifies imports/exports and the profile, then writes runtime/source ZIPs with SHA-256 files under `dist`. Existing release outputs are preserved. `BUILD_INFO.json` records the exact source commit and compiler. If rebuilding with another Rust toolchain, refresh its bundled `third_party/Rust-COPYRIGHT-library.html` before distribution.

Default tests use synthetic data, including a Windows CALL/relay execution test. The optional `postfx-audit` tool reads user-supplied files:

```text
postfx-audit pe <eldenring.exe>
postfx-audit patch <0..7 mask> <raw.gparam> <new-output.gparam>
postfx-audit patch-dir <0..7 mask> <input-directory> <new-output-directory>
```

Mask bits: chromatic aberration = 1, vignette = 2, sharpening = 4. Outputs must be new paths. An optional resolver mutation test accepts `POSTFX_TEST_EXE` and runs with `cargo test --test local_pe -- --ignored --nocapture`. Game samples are not distributed.

## License and credits

Copyright (C) 2026 Luna(a492219408). Original project code: [GPL-3.0-only](LICENSE). See [NOTICE](NOTICE) and [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for Rust notices, format references and credit to the reference mod, SoulsFormats and ModEngine3 contributors. When distributing binaries, provide the matching complete source under the applicable license terms.
