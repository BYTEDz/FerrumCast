// build.rs
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 AZHAR ZOUHIR / BYTEDz

use std::process::Command;

fn main() {
    let git_tag = Command::new("git")
        .args(["describe", "--tags", "--always", "--dirty"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string());

    let version = if git_tag.is_empty() {
        env!("CARGO_PKG_VERSION").to_string()
    } else {
        git_tag
    };

    println!("cargo:rustc-env=FERRUMCAST_BUILD_VERSION={}", version);
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");
}