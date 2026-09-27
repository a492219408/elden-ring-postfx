Elden Ring Native PostFX 1.0.0
Author: Luna(a492219408) | License: GPL-3.0-only

REQUIREMENTS
Windows x64, Elden Ring, and ModEngine3 with early native loading and named
initializers. ModEngine3: https://github.com/garyttierney/me3
No replacement resources, shaders or extra Visual C++ runtime are required.

INSTALL
1. Extract the complete archive to a writable folder; keep the file structure.
2. Disable every variant of the old No Chromatic Aberration no Vignette no
   Sharpening resource mod, if present.
3. Open EldenRingPostFX.me3 with ModEngine3 and launch. It uses offline mode.
4. All three removals are on. Edit natives/EldenRingPostFX.ini to change them.
   Fully exit and restart the game after any configuration change.

true removes the corresponding effect; false preserves the incoming value.
observe_only should normally stay false; true counts without applying changes.
Turning an option off cannot undo a value already changed by another mod.
This mod changes game effects, not ReShade or driver filters.

EXISTING PROFILE
Keep the INI next to the DLL. Add this native entry to your profile and adjust
the path relative to that profile. Preserve load_early and initializer:

[[natives]]
path = 'mods/eldenring/EldenRingPostFX/natives/EldenRingPostFX.dll'
enabled = true
optional = false
load_early = true
initializer = { function = "elden_ring_postfx_init" }

UNINSTALL
Exit the game and disable/remove this native entry. Game files on disk are
unchanged. Hot loading, live configuration reload and hot unload are unsupported.

TROUBLESHOOTING
The log is natives/EldenRingPostFX.log and is replaced on each initialization.
After loading a save, check actual import statistics, not just HOOK INSTALLED.
If initialization fails, check the INI, early loading and exact initializer.
For reports, preserve the log and include game/regulation versions, your INI
and other active mods.

VALIDATION
The same effect/hook logic was tested by the maintainer in preview 0.1.0 on
1.17 and 1.17.1. The formal binary has been rebuilt. See RELEASE_NOTES.txt for
the exact scope. Future versions, all regions and mod combinations are not
guaranteed. No performance improvement is claimed.

SOURCE AND LICENSES
BUILD_INFO.json identifies the exact source commit and matching source ZIP.
The release's source archive is elden-ring-postfx-1.0.0-source.zip.
Keep LICENSE.txt, NOTICE.txt, THIRD_PARTY_NOTICES.txt and the Rust copyright
notice with redistributions, and provide corresponding source as required.
Runtime installation does not require downloading the source archive.
