// Copyright (C) 2026 CRV Science Network (https://crvscience.com)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://gnu.org>.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn parse_version_quad(version: &str) -> (u16, u16, u16, u16) {
    let mut parts = version
        .split(['.', '-', '+'])
        .filter_map(|p| p.parse::<u16>().ok());
    (
        parts.next().unwrap_or(2),
        parts.next().unwrap_or(2),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

fn find_msvc_rc() -> Option<PathBuf> {
    if let Ok(output) = Command::new("where.exe").arg("rc.exe").output() {
        if output.status.success() {
            if let Some(line) = String::from_utf8_lossy(&output.stdout).lines().next() {
                let p = PathBuf::from(line.trim());
                if p.exists() {
                    return Some(p);
                }
            }
        }
    }
    let pf86 = env::var_os("ProgramFiles(x86)").map(PathBuf::from)?;
    let kits_bin = pf86.join("Windows Kits").join("10").join("bin");
    let entries = fs::read_dir(kits_bin).ok()?;
    let mut candidates: Vec<PathBuf> = Vec::new();
    for entry in entries.flatten() {
        let rc = entry.path().join("x64").join("rc.exe");
        if rc.is_file() {
            candidates.push(rc);
        }
    }
    candidates.sort();
    candidates.pop()
}

fn main() {
    println!("cargo:rerun-if-changed=assets/quantacrypt-studio.ico");
    println!("cargo:rerun-if-changed=Cargo.toml");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let ico_path = manifest_dir.join("assets").join("quantacrypt-studio.ico");
    if !ico_path.is_file() {
        println!(
            "cargo:warning=Icon file missing at {}, skipping Windows resource embedding",
            ico_path.display()
        );
        return;
    }

    let pkg_version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "2.2.0".to_string());
    let (v0, v1, v2, v3) = parse_version_quad(&pkg_version);
    let ico_rc = ico_path.to_string_lossy().replace('\\', "/");

    let rc_content = format!(
        r#"1 ICON "{ico_rc}"

1 VERSIONINFO
FILEVERSION {v0},{v1},{v2},{v3}
PRODUCTVERSION {v0},{v1},{v2},{v3}
FILEFLAGSMASK 0x3fL
FILEFLAGS 0x0L
FILEOS 0x40004L
FILETYPE 0x1L
FILESUBTYPE 0x0L
BEGIN
    BLOCK "StringFileInfo"
    BEGIN
        BLOCK "040904B0"
        BEGIN
            VALUE "CompanyName", "CRV Science Network\0"
            VALUE "FileDescription", "QuantaCrypt Studio - Quantum Simulation & Cryptography Workstation\0"
            VALUE "FileVersion", "{pkg_version}\0"
            VALUE "InternalName", "quantacrypt-studio\0"
            VALUE "LegalCopyright", "Copyright CRV Science Network (GPL-3.0-or-later)\0"
            VALUE "OriginalFilename", "quantacrypt-studio.exe\0"
            VALUE "ProductName", "QuantaCrypt-Studio\0"
            VALUE "ProductVersion", "{pkg_version}\0"
        END
    END
    BLOCK "VarFileInfo"
    BEGIN
        VALUE "Translation", 0x0409, 1200
    END
END
"#
    );

    let rc_path = out_dir.join("app.rc");
    fs::write(&rc_path, rc_content).expect("write app.rc");

    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_env == "gnu" {
        let windres = env::var("WINDRES").unwrap_or_else(|_| "windres".to_string());
        let obj_path = out_dir.join("app_res.o");
        match Command::new(&windres)
            .args([
                "-i",
                rc_path.to_string_lossy().as_ref(),
                "-o",
                obj_path.to_string_lossy().as_ref(),
                "-O",
                "coff",
            ])
            .status()
        {
            Ok(status) if status.success() => {
                println!("cargo:rustc-link-arg={}", obj_path.display());
            }
            Ok(status) => {
                panic!("{} failed compiling {} with status {}", windres, rc_path.display(), status);
            }
            Err(err) => {
                println!(
                    "cargo:warning={} not found ({}); skipping Windows PE resource embedding",
                    windres, err
                );
            }
        }
    } else if target_env == "msvc" {
        if let Some(rc_exe) = find_msvc_rc() {
            let res_path = out_dir.join("app_res.res");
            let status = Command::new(&rc_exe)
                .args([
                    "/nologo",
                    "/fo",
                    res_path.to_string_lossy().as_ref(),
                    rc_path.to_string_lossy().as_ref(),
                ])
                .status()
                .expect("run rc.exe");
            if !status.success() {
                panic!("rc.exe failed with status {}", status);
            }
            println!("cargo:rustc-link-arg={}", res_path.display());
        } else {
            println!("cargo:warning=rc.exe not found; skipping Windows PE resource embedding");
        }
    } else {
        let _ = Path::new(&rc_path);
    }
}
