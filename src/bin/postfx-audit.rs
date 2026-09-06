// Copyright (C) 2026 Luna(a492219408). SPDX-License-Identifier: GPL-3.0-only
use elden_ring_postfx::{config::Config, gparam, pe::Image};
use std::{error::Error, fs, io::Write, path::PathBuf};

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    match args.first().and_then(|s| s.to_str()) {
        Some("pe") if args.len() == 2 => {
            let data = fs::read(&args[1])?;
            let image = Image::from_file(&data)?;
            let site = image.resolve()?;
            println!("call_rva=0x{:x} copy_rva=0x{:x} function=0x{:x}..0x{:x}", site.call_rva, site.copy_rva, site.function.start, site.function.end);
        }
        Some("patch") if args.len() == 4 => {
            let mask: u8 = args[1].to_str().ok_or("invalid mask")?.parse()?;
            if mask > 7 { return Err("mask must be 0..7 (CA=1, vignette=2, sharpening=4)".into()); }
            let mut data = fs::read(&args[2])?;
            let report = gparam::patch(&mut data, Config::from_mask(mask)).map_err(|e| format!("GPARAM rejected: {e:?}"))?;
            // Never replace the input or an existing file. This tool does not decompress DCX.
            let mut out = fs::OpenOptions::new().write(true).create_new(true).open(PathBuf::from(&args[3]))?;
            out.write_all(&data)?;
            println!("found={:?} changed={:?}", report.found, report.changed);
        }
        Some("patch-dir") if args.len() == 4 => {
            let mask: u8 = args[1].to_str().ok_or("invalid mask")?.parse()?;
            if mask > 7 { return Err("mask must be 0..7".into()); }
            let input = PathBuf::from(&args[2]);
            let output = PathBuf::from(&args[3]);
            // Directory must be new, preventing accidental in-place or repeated overwrites.
            fs::create_dir(&output)?;
            let mut count = 0;
            let mut found = [0; 3];
            let mut changed = [0; 3];
            for entry in fs::read_dir(input)? {
                let entry = entry?;
                if !entry.file_type()?.is_file() || entry.path().extension().is_none_or(|s| s != "gparam") { continue; }
                let mut data = fs::read(entry.path())?;
                let report = gparam::patch(&mut data, Config::from_mask(mask)).map_err(|e| format!("{}: {e:?}", entry.path().display()))?;
                let mut out = fs::OpenOptions::new().write(true).create_new(true).open(output.join(entry.file_name()))?;
                out.write_all(&data)?;
                for i in 0..3 { found[i] += report.found[i]; changed[i] += report.changed[i]; }
                count += 1;
            }
            if count == 0 { return Err("no raw GPARAM files found".into()); }
            println!("files={count} found={found:?} changed={changed:?}");
        }
        _ => return Err("Usage: postfx-audit pe <eldenring.exe> | patch <0..7 mask> <raw.gparam> <new-output.gparam> | patch-dir <mask> <input-directory> <new-output-directory>".into()),
    }
    Ok(())
}
