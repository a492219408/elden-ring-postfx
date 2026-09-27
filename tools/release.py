"""Check the public boundary and build matching runtime/source release archives.

Copyright (C) 2026 Luna(a492219408). SPDX-License-Identifier: GPL-3.0-only
Python 3.11+, standard library only. No private workspace or game files required.
"""
import argparse
import hashlib
import json
import re
import shutil
import struct
import subprocess
import tomllib
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TARGET = "x86_64-pc-windows-msvc"
FORBIDDEN = re.compile(r"(?i)(^|/)(AGENTS\.md|\.git|local|state|evidence|publishing|analysis|视觉测试)(/|$)|\.(dcx|gparam|gfx|tpf|dds|png|jpg|zip|log|pdb|dll|exe)$")


def run(*args, binary=False, show=False):
    command = ["mise", "exec", "--", *map(str, args)] if shutil.which("mise") else list(map(str, args))
    proc = subprocess.run(command, cwd=ROOT, check=True, capture_output=not show)
    return None if show else proc.stdout if binary else proc.stdout.decode("utf-8").strip()


def sha(data):
    return hashlib.sha256(data).hexdigest()


def check():
    allowed = (ROOT / "packaging/source-files.txt").read_text(encoding="utf-8").splitlines()
    if len(allowed) != len(set(allowed)) or any(not p or FORBIDDEN.search(p) or ".." in Path(p).parts or p.startswith(("/", "\\")) or ":" in p for p in allowed):
        raise ValueError("Invalid source allowlist")
    if Path(run("git", "rev-parse", "--show-toplevel")).resolve() != ROOT:
        raise ValueError("Packaging requires this project's own Git checkout")
    if run("git", "status", "--porcelain"):
        raise ValueError("Commit source changes before packaging")
    tracked = set(run("git", "ls-files").splitlines())
    if tracked != set(allowed):
        raise ValueError(f"Source allowlist mismatch: {sorted(tracked ^ set(allowed))}")
    revisions = run("git", "rev-list", "--all", "HEAD").splitlines()
    for revision in revisions:
        for entry in run("git", "ls-tree", "-r", revision).splitlines():
            meta, path = entry.split("\t", 1)
            if path not in allowed or meta.split()[0] not in ("100644", "100755"):
                raise ValueError(f"Non-public path or special file in reachable history: {revision} {path}")
    run("git", "diff", "--check")
    return {"commit": run("git", "rev-parse", "HEAD"), "source_files": allowed, "reachable_commits_checked": len(revisions)}


def inspect_dll(path):
    data = path.read_bytes()
    pe = struct.unpack_from("<I", data, 0x3c)[0]
    if data[:2] != b"MZ" or data[pe:pe+4] != b"PE\0\0" or struct.unpack_from("<H", data, pe+4)[0] != 0x8664:
        raise ValueError("Expected AMD64 DLL")
    opt, count = pe+24, struct.unpack_from("<H", data, pe+6)[0]
    opt_size = struct.unpack_from("<H", data, pe+20)[0]
    sections = [struct.unpack_from("<4I", data, opt+opt_size+i*40+8) for i in range(count)]
    def offset(rva):
        for _, va, size, raw in sections:
            if va <= rva < va+size:
                return raw+rva-va
        raise ValueError("Unmapped PE RVA")
    def string(rva):
        pos = offset(rva)
        return data[pos:data.index(b"\0", pos)].decode("ascii")
    exp, _, imp, _ = struct.unpack_from("<4I", data, opt+112)
    pos = offset(exp)
    names_count, names_rva = struct.unpack_from("<I", data, pos+24)[0], struct.unpack_from("<I", data, pos+32)[0]
    names = offset(names_rva)
    exports = [string(struct.unpack_from("<I", data, names+i*4)[0]) for i in range(names_count)]
    imports, pos = [], offset(imp)
    while any(data[pos:pos+20]):
        imports.append(string(struct.unpack_from("<I", data, pos+12)[0]))
        pos += 20
    if set(exports) != {"DllMain", "elden_ring_postfx_init"}:
        raise ValueError(f"Unexpected DLL exports: {exports}")
    if any(not (name.lower() in {"kernel32.dll", "ntdll.dll", "advapi32.dll", "bcryptprimitives.dll", "userenv.dll", "ws2_32.dll", "dbghelp.dll"} or name.lower().startswith(("api-ms-win-", "ext-ms-win-"))) for name in imports):
        raise ValueError(f"Unexpected runtime DLL dependency: {imports}")
    return {"bytes": len(data), "sha256": sha(data), "exports": exports, "imports": imports}


def verify_runtime(package):
    expected = {"natives/EldenRingPostFX.dll", "natives/EldenRingPostFX.ini", "EldenRingPostFX.me3", "README.txt", "README.zh-CN.txt", "LICENSE.txt", "NOTICE.txt", "THIRD_PARTY_NOTICES.txt", "Rust-COPYRIGHT-library.html", "RELEASE_NOTES.txt", "BUILD_INFO.json"}
    actual = {p.relative_to(package).as_posix() for p in package.rglob("*") if p.is_file()}
    if actual != expected:
        raise ValueError(f"Unexpected runtime contents: {sorted(actual ^ expected)}")
    profile = tomllib.loads((package / "EldenRingPostFX.me3").read_text(encoding="utf-8-sig"))
    natives = profile["natives"]
    if profile["start_online"] is not False or "packages" in profile or len(natives) != 1:
        raise ValueError("Expected offline single-native profile")
    native = natives[0]
    if native["path"] != "natives/EldenRingPostFX.dll" or native["enabled"] is not True or native["optional"] is not False or native["load_early"] is not True or native["initializer"] != {"function": "elden_ring_postfx_init"}:
        raise ValueError("Invalid native initializer configuration")
    result = inspect_dll(package / "natives/EldenRingPostFX.dll")
    info = json.loads((package / "BUILD_INFO.json").read_text(encoding="utf-8"))
    if info["dll"]["sha256"] != result["sha256"]:
        raise ValueError("DLL differs from build manifest")
    return result


def write_hash(path):
    path.with_name(path.name + ".sha256").write_text(f"{sha(path.read_bytes())}  {path.name}\n", encoding="ascii")


def pack():
    boundary = check()
    version = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["package"]["version"]
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        raise ValueError("This release script expects a stable version")
    name = f"EldenRingPostFX-{version}"
    source_name = f"elden-ring-postfx-{version}-source"
    dist, package = ROOT / "dist", ROOT / "dist" / name
    runtime, source = dist / (name + ".zip"), dist / (source_name + ".zip")
    if any(p.exists() for p in (package, runtime, source, runtime.with_suffix(".zip.sha256"), source.with_suffix(".zip.sha256"))):
        raise ValueError("Release output exists; preserve it and use a new version or a fresh checkout")
    toolchain_notice = Path(run("rustc", "--print", "sysroot")) / "share/doc/rust/COPYRIGHT-library.html"
    if toolchain_notice.read_bytes() != (ROOT / "third_party/Rust-COPYRIGHT-library.html").read_bytes():
        raise ValueError("Refresh the Rust standard library notice for the active toolchain")
    run("pwsh", "-NoProfile", "-File", ROOT / "scripts/Build.ps1", show=True)
    if check()["commit"] != boundary["commit"]:
        raise ValueError("Source changed while building")
    dll = ROOT / "target" / TARGET / "release/elden_ring_postfx.dll"
    binary_info = inspect_dll(dll)
    if f"EldenRingPostFX {version}".encode() not in dll.read_bytes():
        raise ValueError("DLL release version is missing")
    (package / "natives").mkdir(parents=True)
    copies = {
        dll: "natives/EldenRingPostFX.dll",
        ROOT / "assets/EldenRingPostFX.ini": "natives/EldenRingPostFX.ini",
        ROOT / "assets/EldenRingPostFX.me3": "EldenRingPostFX.me3",
        ROOT / "packaging/release/README.txt": "README.txt",
        ROOT / "packaging/release/README.zh-CN.txt": "README.zh-CN.txt",
        ROOT / "LICENSE": "LICENSE.txt", ROOT / "NOTICE": "NOTICE.txt",
        ROOT / "THIRD_PARTY_NOTICES.md": "THIRD_PARTY_NOTICES.txt",
        ROOT / "third_party/Rust-COPYRIGHT-library.html": "Rust-COPYRIGHT-library.html",
        ROOT / "RELEASE_NOTES.md": "RELEASE_NOTES.txt",
    }
    for origin, destination in copies.items():
        shutil.copyfile(origin, package / destination)
    info = {"version": version, "source_commit": boundary["commit"], "source_tag": f"v{version}", "source_archive": source.name, "target": TARGET, "rustc": run("rustc", "--version"), "dll": binary_info}
    (package / "BUILD_INFO.json").write_text(json.dumps(info, indent=2) + "\n", encoding="utf-8")
    verify_runtime(package)
    run("git", "archive", "--format=zip", f"--prefix={source_name}/", f"--output={source}", "HEAD")
    with zipfile.ZipFile(source) as archive:
        actual = {n[len(source_name)+1:] for n in archive.namelist() if not n.endswith("/")}
        if actual != set(boundary["source_files"]):
            raise ValueError("Source ZIP differs from allowlist")
    with zipfile.ZipFile(runtime, "x", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for path in sorted(package.rglob("*")):
            if path.is_file():
                archive.write(path, name + "/" + path.relative_to(package).as_posix())
    for path in (runtime, source):
        write_hash(path)
    check()
    print(json.dumps({"runtime": str(runtime), "source": str(source), "build": info, "boundary": {k:v for k,v in boundary.items() if k != "source_files"}}, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("check", "pack", "verify"))
    parser.add_argument("--package", type=Path)
    args = parser.parse_args()
    if args.action == "pack":
        pack()
    elif args.action == "check":
        print(json.dumps(check(), indent=2))
    else:
        if args.package is None:
            parser.error("verify requires --package")
        print(json.dumps(verify_runtime(args.package), indent=2))
