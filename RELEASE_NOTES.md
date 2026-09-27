# Elden Ring Native PostFX 1.0.0

First stable release of the configurable ModEngine3 Native DLL.

- Independently disable chromatic aberration, vignette and built-in sharpening through one INI file. All three are enabled by default; restart the game after changing settings.
- Apply changes during GPARAM import without replacement game resources or per-frame scanning, preserving unrelated game and parameter-mod values.
- Keep the effect and hook implementation from `0.1.0-preview-2b1f244`. The maintainer ran that preview on Elden Ring 1.17.1 / Regulation 1.17.1 and reported no issues; its log recorded all three modifications with no parser rejections or caught panics.
- Add English and Simplified Chinese instructions, bundled Rust license notices, matching source archives and SHA-256 checksums.

The earlier 1.17 screenshots demonstrate the three effects in one scene. Older 1.16.2 / 1.16.1 support has offline evidence only. Unknown future updates and untested areas/mod combinations remain unverified. This release is a newly built binary and has not itself been launched in the game by the maintainer at packaging time.

Requires Windows x64, Elden Ring and ModEngine3 with early native loading and named initializer support. Use the supplied offline profile or merge its native entry into your own. Disable all variants of the old resource mod. This DLL does not remove sharpening from ReShade or graphics drivers.

Author: Luna(a492219408). License: GPL-3.0-only.
