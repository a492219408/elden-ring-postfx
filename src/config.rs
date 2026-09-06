// Copyright (C) 2026 Luna(a492219408). SPDX-License-Identifier: GPL-3.0-only
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    pub disable_chromatic_aberration: bool,
    pub disable_vignette: bool,
    pub disable_sharpening: bool,
    pub observe_only: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self::from_mask(7)
    }
}

impl Config {
    pub const fn from_mask(mask: u8) -> Self {
        Self {
            disable_chromatic_aberration: mask & 1 != 0,
            disable_vignette: mask & 2 != 0,
            disable_sharpening: mask & 4 != 0,
            observe_only: false,
        }
    }

    pub fn enabled(self) -> [bool; 3] {
        [
            self.disable_chromatic_aberration,
            self.disable_vignette,
            self.disable_sharpening,
        ]
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let mut result = Self::default();
        let mut section = false;
        let mut seen = [false; 4];
        for (line, raw) in text.trim_start_matches('\u{feff}').lines().enumerate() {
            let s = raw.split([';', '#']).next().unwrap_or("").trim();
            if s.is_empty() {
                continue;
            }
            let fail = || format!("invalid configuration at line {}: {s}", line + 1);
            if s.starts_with('[') {
                if s != "[postfx]" || section {
                    return Err(fail());
                }
                section = true;
                continue;
            }
            if !section {
                return Err(fail());
            }
            let (key, value) = s.split_once('=').ok_or_else(fail)?;
            let index = match key.trim() {
                "disable_chromatic_aberration" => 0,
                "disable_vignette" => 1,
                "disable_sharpening" => 2,
                "observe_only" => 3,
                _ => return Err(fail()),
            };
            if std::mem::replace(&mut seen[index], true) {
                return Err(fail());
            }
            let value = match value.trim() {
                "true" => true,
                "false" => false,
                _ => return Err(fail()),
            };
            match index {
                0 => result.disable_chromatic_aberration = value,
                1 => result.disable_vignette = value,
                2 => result.disable_sharpening = value,
                _ => result.observe_only = value,
            }
        }
        if !section {
            return Err("missing [postfx] section".into());
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_are_independent_and_typos_fail() {
        let c = Config::parse(
            "\u{feff}[postfx]\ndisable_vignette=false ; comment\nobserve_only=true\n",
        )
        .unwrap();
        assert_eq!(c.enabled(), [true, false, true]);
        assert!(c.observe_only);
        for invalid in [
            "",
            "[unknown]",
            "[postfx]\ndisable_vignette=0",
            "[postfx]\ndisable_vignete=true",
            "[postfx]\nobserve_only=true\nobserve_only=false",
        ] {
            assert!(Config::parse(invalid).is_err(), "{invalid}");
        }
    }
}
