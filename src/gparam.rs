// Copyright (C) 2026 Luna(a492219408). SPDX-License-Identifier: GPL-3.0-only
// Format reference: SoulsFormatsNEXT GPARAM.cs; no game assets are compiled in.
use crate::config::Config;
use std::ops::Range;

pub const MAX_SIZE: usize = 16 * 1024 * 1024;
const MAGIC: &[u8; 8] = b"f\0i\0l\0t\0";
const TARGETS: [(&str, &str); 3] = [
    (
        "Tone Map[Yebis]",
        "GS Presents - Chromatic Aberration Enable",
    ),
    ("Tone Map[Yebis]", "GS Presents - Vignetting Power"),
    ("Anti-Alias-Editor", "AntiAlias - SharpenIntensity"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    NotGparam,
    UnsupportedVersion,
    Bounds,
    Layout,
    TooComplex,
    FieldType,
    Overlap,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// Recognized rows, including disabled options and values that are already zero.
    pub found: [usize; 3],
    /// Nonzero rows selected by the configuration (would change in observe mode).
    pub changed: [usize; 3],
}

struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn bytes(&self, offset: usize, n: usize) -> Result<&'a [u8], Error> {
        self.0
            .get(offset..offset.checked_add(n).ok_or(Error::Bounds)?)
            .ok_or(Error::Bounds)
    }
    fn u32(&self, offset: usize) -> Result<usize, Error> {
        Ok(u32::from_le_bytes(
            self.bytes(offset, 4)?
                .try_into()
                .map_err(|_| Error::Bounds)?,
        ) as usize)
    }
    fn string(&self, start: usize, end: usize) -> Result<(&'a [u8], usize), Error> {
        if !start.is_multiple_of(2) || end < start {
            return Err(Error::Layout);
        }
        let limit = end.min(start.checked_add(8192).ok_or(Error::Bounds)?);
        let bytes = self.bytes(start, limit - start)?;
        let len = bytes
            .chunks_exact(2)
            .position(|c| c == [0, 0])
            .ok_or(Error::Layout)?
            * 2;
        Ok((&bytes[..len], start + len + 2))
    }
}

fn ascii_eq(wide: &[u8], name: &str) -> bool {
    wide.len() == name.len() * 2
        && wide
            .chunks_exact(2)
            .zip(name.bytes())
            .all(|(w, b)| w == [b, 0])
}

fn within(start: usize, length: usize, range: &Range<usize>) -> Result<(), Error> {
    let end = start.checked_add(length).ok_or(Error::Bounds)?;
    if start < range.start || end > range.end {
        Err(Error::Bounds)
    } else {
        Ok(())
    }
}

fn nonoverlapping(spans: &mut [Range<usize>]) -> Result<(), Error> {
    spans.sort_unstable_by_key(|s| s.start);
    if spans.windows(2).any(|s| s[0].end > s[1].start) {
        Err(Error::Overlap)
    } else {
        Ok(())
    }
}

/// Validate the complete relevant structure before changing any bytes. Edits never
/// change lengths, ValueIDs, times, comments or other fields. Every value row is used.
pub fn patch(data: &mut [u8], config: Config) -> Result<Report, Error> {
    let (offsets, report) = plan(data, config)?;
    if !config.observe_only {
        for offset in offsets {
            data[offset..offset + 4].fill(0);
        }
    }
    Ok(report)
}

fn plan(data: &[u8], config: Config) -> Result<(Vec<usize>, Report), Error> {
    let r = Reader(data);
    if data.len() < 0x54 || data.len() > MAX_SIZE || data.get(..8) != Some(MAGIC) {
        return Err(Error::NotGparam);
    }
    if r.u32(8)? != 5 {
        return Err(Error::UnsupportedVersion);
    }
    if r.u32(0x18)? != 0x54 {
        return Err(Error::Layout);
    }
    let group_count = r.u32(0x10)?;
    if group_count > 1024 {
        return Err(Error::TooComplex);
    }
    let mut sections = Vec::new();
    for offset in [
        0x18, 0x1c, 0x20, 0x24, 0x28, 0x2c, 0x30, 0x38, 0x3c, 0x44, 0x48, 0x4c,
    ] {
        sections.push(r.u32(offset)?);
    }
    sections.push(data.len());
    if sections.windows(2).any(|s| s[0] > s[1]) {
        return Err(Error::Layout);
    }
    let groups = sections[1]..sections[2];
    let tables = sections[2]..sections[3];
    let params = sections[3]..sections[4];
    let values = sections[4]..sections[5];
    let ids = sections[5]..sections[6];
    within(0x54, group_count * 4, &(0x54..groups.start))?;
    let mut offsets = Vec::new();
    let mut value_spans = Vec::new();
    let mut id_spans = Vec::new();
    let mut report = Report::default();
    let mut total_fields = 0;
    for gi in 0..group_count {
        let gp = groups
            .start
            .checked_add(r.u32(0x54 + gi * 4)?)
            .ok_or(Error::Bounds)?;
        within(gp, 8, &groups)?;
        let count = r.u32(gp)?;
        total_fields += count;
        if total_fields > 8192 {
            return Err(Error::TooComplex);
        }
        let table = tables
            .start
            .checked_add(r.u32(gp + 4)?)
            .ok_or(Error::Bounds)?;
        within(table, count * 4, &tables)?;
        let (group_name, next) = r.string(gp + 8, groups.end)?;
        r.string(next, groups.end)?;
        for pi in 0..count {
            let pp = params
                .start
                .checked_add(r.u32(table + pi * 4)?)
                .ok_or(Error::Bounds)?;
            within(pp, 12, &params)?;
            let header = r.bytes(pp, 12)?;
            let ty = header[8];
            let n = header[9] as usize;
            if header[10..12] != [0, 0] {
                return Err(Error::Layout);
            }
            let width = match ty {
                1 | 5 | 11 => 1,
                2 => 2,
                3 | 7 | 9 | 15 => 4,
                12..=14 => 16,
                _ => return Err(Error::FieldType),
            };
            let vp = values.start.checked_add(r.u32(pp)?).ok_or(Error::Bounds)?;
            let ip = ids.start.checked_add(r.u32(pp + 4)?).ok_or(Error::Bounds)?;
            within(vp, n * width, &values)?;
            within(ip, n * 8, &ids)?;
            if n > 0 {
                value_spans.push(vp..vp + n * width);
                id_spans.push(ip..ip + n * 8);
            }
            let (name, next) = r.string(pp + 12, params.end)?;
            r.string(next, params.end)?;
            let Some(effect) = TARGETS
                .iter()
                .position(|(g, f)| ascii_eq(group_name, g) && ascii_eq(name, f))
            else {
                continue;
            };
            if (effect == 0 && ![3, 7].contains(&ty)) || (effect != 0 && ty != 9) || vp % 4 != 0 {
                return Err(Error::FieldType);
            }
            report.found[effect] += n;
            if config.enabled()[effect] {
                for vi in 0..n {
                    let offset = vp + vi * 4;
                    if r.bytes(offset, 4)? != [0, 0, 0, 0] {
                        offsets.push(offset);
                        report.changed[effect] += 1;
                    }
                }
            }
        }
    }
    nonoverlapping(&mut value_spans)?;
    nonoverlapping(&mut id_spans)?;
    Ok((offsets, report))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub(crate) fn put(data: &mut [u8], at: usize, value: usize) {
        data[at..at + 4].copy_from_slice(&(value as u32).to_le_bytes());
    }
    fn wide(data: &mut Vec<u8>, value: &str) {
        data.extend(value.encode_utf16().chain([0]).flat_map(u16::to_le_bytes));
    }
    fn align(data: &mut Vec<u8>) {
        data.resize(data.len().next_multiple_of(4), 0);
    }

    // Entirely synthetic: names identify the public interface, all values/IDs are invented.
    pub(crate) fn fixture() -> (Vec<u8>, Vec<(usize, usize)>, Vec<usize>) {
        let mut data = vec![0; 0x54 + 12];
        data[..8].copy_from_slice(MAGIC);
        put(&mut data, 8, 5);
        put(&mut data, 0xc, 0x100);
        put(&mut data, 0x10, 3);
        put(&mut data, 0x18, 0x54);
        let groups = data.len();
        put(&mut data, 0x1c, groups);
        let definitions = [
            ("Tone Map[Yebis]", vec![0, 1]),
            ("Anti-Alias-Editor", vec![2]),
            ("Unrelated", vec![0]),
        ];
        let mut field_count = 0;
        for (gi, (name, fields)) in definitions.iter().enumerate() {
            let gp = data.len();
            put(&mut data, 0x54 + gi * 4, gp - groups);
            data.resize(gp + 8, 0);
            put(&mut data, gp, fields.len());
            put(&mut data, gp + 4, field_count * 4);
            wide(&mut data, name);
            wide(&mut data, "label");
            align(&mut data);
            field_count += fields.len();
        }
        let tables = data.len();
        put(&mut data, 0x20, tables);
        data.resize(tables + field_count * 4, 0);
        let params = data.len();
        put(&mut data, 0x24, params);
        let mut headers = Vec::new();
        for (i, effect) in [0, 1, 2, 0].into_iter().enumerate() {
            let p = data.len();
            put(&mut data, tables + i * 4, p - params);
            headers.push(p);
            data.resize(p + 12, 0);
            put(&mut data, p, i * 12);
            put(&mut data, p + 4, i * 24);
            data[p + 8] = if effect == 0 { 7 } else { 9 };
            data[p + 9] = 3;
            wide(&mut data, TARGETS[effect].1);
            wide(&mut data, "field label");
            align(&mut data);
        }
        let values = data.len();
        put(&mut data, 0x28, values);
        let mut targets = Vec::new();
        for effect in 0..4 {
            for row in 0..3 {
                if effect < 3 {
                    targets.push((effect, data.len()));
                }
                data.extend(
                    (if effect == 0 || effect == 3 {
                        row + 1
                    } else {
                        (0.25 * (row + 1) as f32).to_bits()
                    })
                    .to_le_bytes(),
                );
            }
        }
        let ids = data.len();
        put(&mut data, 0x2c, ids);
        for _ in 0..4 {
            for row in 0..3u32 {
                data.extend((101 + row).to_le_bytes());
                data.extend((row as f32 * 8.0).to_le_bytes());
            }
        }
        let end = data.len();
        for o in [0x30, 0x38, 0x3c, 0x44, 0x48, 0x4c] {
            put(&mut data, o, end);
        }
        (data, targets, headers)
    }

    #[test]
    fn all_eight_combinations_edit_only_selected_rows_and_are_idempotent() {
        let (original, targets, _) = fixture();
        for mask in 0..8 {
            let mut actual = original.clone();
            let mut expected = original.clone();
            for &(effect, offset) in &targets {
                if mask & (1 << effect) != 0 {
                    expected[offset..offset + 4].fill(0);
                }
            }
            let report = patch(&mut actual, Config::from_mask(mask)).unwrap();
            assert_eq!(actual, expected, "mask {mask}");
            assert_eq!(report.found, [3, 3, 3]);
            assert_eq!(
                report.changed,
                std::array::from_fn(|i| if mask & (1 << i) != 0 { 3 } else { 0 })
            );
            assert_eq!(
                patch(&mut actual, Config::from_mask(mask)).unwrap().changed,
                [0; 3]
            );
        }
    }

    #[test]
    fn observation_and_signed_chromatic_type() {
        let (mut data, _, headers) = fixture();
        data[headers[0] + 8] = 3;
        let original = data.clone();
        let config = Config {
            observe_only: true,
            ..Config::default()
        };
        assert_eq!(patch(&mut data, config).unwrap().changed, [3; 3]);
        assert_eq!(data, original);
        assert_eq!(patch(&mut data, Config::default()).unwrap().changed, [3; 3]);
    }

    #[test]
    fn malformed_late_field_and_overlap_never_partially_edit() {
        let (original, _, headers) = fixture();
        let mut malformed = original.clone();
        malformed[headers[2] + 8] = 7;
        let before = malformed.clone();
        assert_eq!(
            patch(&mut malformed, Config::default()),
            Err(Error::FieldType)
        );
        assert_eq!(malformed, before);
        let mut overlap = original;
        put(&mut overlap, headers[1], 0);
        let before = overlap.clone();
        assert_eq!(patch(&mut overlap, Config::default()), Err(Error::Overlap));
        assert_eq!(overlap, before);
    }

    #[test]
    fn truncations_and_structural_mutations_are_bounded_and_transactional() {
        let (original, _, _) = fixture();
        for length in 0..original.len() {
            let mut data = original[..length].to_vec();
            let before = data.clone();
            assert!(patch(&mut data, Config::default()).is_err());
            assert_eq!(data, before);
        }
        for at in (0..original.len()).step_by(3) {
            let mut data = original.clone();
            data[at] ^= 0xff;
            let before = data.clone();
            if patch(&mut data, Config::default()).is_err() {
                assert_eq!(data, before);
            }
        }
    }
}
