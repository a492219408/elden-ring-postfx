// Copyright (C) 2026 Luna(a492219408). SPDX-License-Identifier: GPL-3.0-only
// Short instruction patterns identify a single GPARAM import copy, not global memcpy.
use std::ops::Range;

#[derive(Clone, Debug)]
pub struct Section {
    pub name: String,
    pub rva: usize,
    pub virtual_size: usize,
    pub raw_offset: usize,
    pub raw_size: usize,
    pub flags: u32,
}

#[derive(Debug)]
pub struct Headers {
    pub image_size: usize,
    pub exception_rva: usize,
    pub exception_size: usize,
    pub sections: Vec<Section>,
}

#[derive(Debug)]
pub struct Image {
    pub headers: Headers,
    pub sections: Vec<(Section, Vec<u8>)>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Site {
    pub call_rva: usize,
    pub copy_rva: usize,
    pub function: Range<usize>,
    pub original_call: [u8; 5],
}

fn bytes(data: &[u8], offset: usize, n: usize) -> Result<&[u8], String> {
    data.get(offset..offset.checked_add(n).ok_or("PE offset overflow")?)
        .ok_or_else(|| "PE bounds".into())
}
fn u16_at(data: &[u8], offset: usize) -> Result<u16, String> {
    Ok(u16::from_le_bytes(
        bytes(data, offset, 2)?.try_into().map_err(|_| "u16")?,
    ))
}
fn u32_at(data: &[u8], offset: usize) -> Result<usize, String> {
    Ok(u32::from_le_bytes(bytes(data, offset, 4)?.try_into().map_err(|_| "u32")?) as usize)
}

impl Headers {
    pub fn parse(data: &[u8]) -> Result<Self, String> {
        if bytes(data, 0, 2)? != b"MZ" {
            return Err("not a PE image".into());
        }
        let pe = u32_at(data, 0x3c)?;
        if pe > 0x800 || bytes(data, pe, 4)? != b"PE\0\0" || u16_at(data, pe + 4)? != 0x8664 {
            return Err("expected AMD64 PE".into());
        }
        let count = u16_at(data, pe + 6)? as usize;
        let opt = pe + 24;
        let opt_size = u16_at(data, pe + 20)? as usize;
        if u16_at(data, opt)? != 0x20b
            || opt_size < 0xf0
            || count > 32
            || u32_at(data, opt + 108)? < 4
        {
            return Err("unsupported PE headers".into());
        }
        let image_size = u32_at(data, opt + 56)?;
        if !(0x1000..=512 * 1024 * 1024).contains(&image_size) {
            return Err("PE image size".into());
        }
        let mut sections = Vec::new();
        for i in 0..count {
            let pos = opt + opt_size + i * 40;
            let name_bytes = bytes(data, pos, 8)?;
            let name =
                String::from_utf8_lossy(name_bytes.split(|c| *c == 0).next().unwrap_or_default())
                    .into_owned();
            let s = Section {
                name,
                rva: u32_at(data, pos + 12)?,
                virtual_size: u32_at(data, pos + 8)?,
                raw_offset: u32_at(data, pos + 20)?,
                raw_size: u32_at(data, pos + 16)?,
                flags: u32_at(data, pos + 36)? as u32,
            };
            if s.rva + s.virtual_size.max(s.raw_size) > image_size
                || sections.iter().any(|p: &Section| {
                    p.rva < s.rva + s.virtual_size && s.rva < p.rva + p.virtual_size
                })
            {
                return Err("overlapping or out-of-bounds PE section".into());
            }
            sections.push(s);
        }
        Ok(Self {
            image_size,
            exception_rva: u32_at(data, opt + 136)?,
            exception_size: u32_at(data, opt + 140)?,
            sections,
        })
    }
}

pub fn relevant(s: &Section) -> bool {
    s.flags & 0x20000000 != 0 || s.name == ".rdata" || s.name == ".pdata"
}

impl Image {
    pub fn from_file(data: &[u8]) -> Result<Self, String> {
        let headers = Headers::parse(data)?;
        let mut sections = Vec::new();
        for s in headers.sections.iter().filter(|s| relevant(s)) {
            sections.push((s.clone(), bytes(data, s.raw_offset, s.raw_size)?.to_vec()));
        }
        Ok(Self { headers, sections })
    }

    fn at(&self, rva: usize, n: usize) -> Result<&[u8], String> {
        self.sections
            .iter()
            .find_map(|(s, d)| rva.checked_sub(s.rva).and_then(|off| bytes(d, off, n).ok()))
            .ok_or_else(|| format!("unmapped RVA {rva:x}"))
    }

    fn executable(&self, rva: usize, n: usize) -> bool {
        self.sections.iter().any(|(s, d)| {
            s.flags & 0x20000000 != 0
                && rva >= s.rva
                && rva.checked_add(n).is_some_and(|end| end <= s.rva + d.len())
        })
    }

    fn rel32(&self, displacement_rva: usize) -> Result<usize, String> {
        let delta = i32::from_le_bytes(
            self.at(displacement_rva, 4)?
                .try_into()
                .map_err(|_| "rel32")?,
        );
        displacement_rva
            .checked_add(4)
            .and_then(|v| v.checked_add_signed(delta as isize))
            .ok_or_else(|| "relative address overflow".into())
    }

    pub fn resolve(&self) -> Result<Site, String> {
        let pattern = Pattern::parse(
            "4c 8b c6 49 8b d6 48 8b c8 e8 ?? ?? ?? ?? 89 74 24 58 45 33 c0 48 8d 15 ?? ?? ?? ?? 48 8d 4c 24 60 e8 ?? ?? ?? ?? 41 b8 10 00 00 00 48 8d 15 ?? ?? ?? ?? 48 8d 4d 90 e8 ?? ?? ?? ??",
        )?;
        let mut candidates = Vec::new();
        for (s, data) in &self.sections {
            if s.flags & 0x20000000 == 0 {
                continue;
            }
            for index in pattern.find_all(data) {
                candidates.push(s.rva + index + 9);
            }
        }
        if candidates.len() != 1 {
            return Err(format!(
                "expected one import copy signature, found {}",
                candidates.len()
            ));
        }
        let call = candidates[0];
        let exceptions = self.at(self.headers.exception_rva, self.headers.exception_size)?;
        if exceptions.len() % 12 != 0 {
            return Err("invalid PE exception directory".into());
        }
        let mut functions = Vec::new();
        for entry in exceptions.chunks_exact(12) {
            let start = u32_at(entry, 0)?;
            let end = u32_at(entry, 4)?;
            if start <= call && call + 51 <= end {
                functions.push(start..end);
            }
        }
        if functions.len() != 1 {
            return Err("ambiguous import function boundary".into());
        }
        let function = functions.remove(0);
        if call - function.start != 0x12d
            || !self.executable(function.start, function.end - function.start)
        {
            return Err("import function layout changed".into());
        }
        let prologue = Pattern::parse(
            "40 55 53 56 57 41 56 48 8d ac 24 c0 fc ff ff 48 81 ec 40 04 00 00 48 c7 44 24 48 fe ff ff ff 48 8b 05 ?? ?? ?? ?? 48 33 c4 48 89 85 30 03 00 00 41 8b f0 4c 8b f2 48 8b d9",
        )?;
        if !prologue.matches(self.at(function.start, prologue.len())?) {
            return Err("import prologue changed".into());
        }
        let allocation = Pattern::parse(
            "48 8b 0b 48 8b 01 41 b8 04 00 00 00 48 8b d6 ff 50 50 48 89 44 24 50 48 85 c0 75 58",
        )?;
        if !allocation.matches(self.at(call - 0x7d, allocation.len())?) {
            return Err("owned-buffer allocation changed".into());
        }
        let lut = self.rel32(call + 38)?;
        let expected: Vec<u8> = "ColorGradingLut_\0"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        if self.at(lut, expected.len())? != expected {
            return Err("GPARAM LUT anchor mismatch".into());
        }
        for name in [
            "GS Presents - Chromatic Aberration Enable",
            "GS Presents - Vignetting Power",
            "AntiAlias - SharpenIntensity",
        ] {
            let wide: Vec<u8> = format!("{name}\0")
                .encode_utf16()
                .flat_map(u16::to_le_bytes)
                .collect();
            let count: usize = self
                .sections
                .iter()
                .filter(|(s, _)| s.name == ".rdata")
                .map(|(_, d)| d.windows(wide.len()).filter(|w| *w == wide).count())
                .sum();
            if count != 1 {
                return Err(format!("field anchor mismatch: {name} ({count})"));
            }
        }
        let copy = self.rel32(call + 1)?;
        let copy_prefix = Pattern::parse(
            "4c 8b d9 4c 8b d2 49 83 f8 10 0f 86 70 00 00 00 49 83 f8 20 76 4a 48 2b d1 73 0f 49 8b c2 49 03 c0 48 3b c8",
        )?;
        if !self.executable(copy, copy_prefix.len())
            || !copy_prefix.matches(self.at(copy, copy_prefix.len())?)
        {
            return Err("copy routine ABI signature changed".into());
        }
        Ok(Site {
            call_rva: call,
            copy_rva: copy,
            function,
            original_call: self.at(call, 5)?.try_into().map_err(|_| "call length")?,
        })
    }
}

struct Pattern(Vec<Option<u8>>);
impl Pattern {
    fn parse(value: &str) -> Result<Self, String> {
        value
            .split_whitespace()
            .map(|v| {
                if v == "??" {
                    Ok(None)
                } else {
                    u8::from_str_radix(v, 16)
                        .map(Some)
                        .map_err(|e| e.to_string())
                }
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Self)
    }
    fn len(&self) -> usize {
        self.0.len()
    }
    fn matches(&self, data: &[u8]) -> bool {
        data.len() == self.len()
            && self
                .0
                .iter()
                .zip(data)
                .all(|(a, b)| a.is_none_or(|a| a == *b))
    }
    fn find_all(&self, data: &[u8]) -> Vec<usize> {
        // Most candidate starts fail the first byte; avoid allocating per comparison.
        data.windows(self.len())
            .enumerate()
            .filter_map(|(i, d)| self.matches(d).then_some(i))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn masked_matching_keeps_all_hits_for_ambiguity_detection() {
        let p = Pattern::parse("48 ?? e8").unwrap();
        assert_eq!(p.find_all(&[0x48, 1, 0xe8, 0x48, 2, 0xe8]), [0, 3]);
        assert!(p.find_all(&[0x48, 1]).is_empty());
        assert!(Headers::parse(&[0; 4096]).is_err());
    }
}
