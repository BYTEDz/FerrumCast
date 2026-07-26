use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=GITHUB_REF_NAME");

    let version = if let Ok(ref_name) = std::env::var("GITHUB_REF_NAME") {
        let v = ref_name.trim();
        if v.starts_with('v') {
            v[1..].to_string()
        } else {
            v.to_string()
        }
    } else {
        Command::new("git")
            .args(["describe", "--tags", "--always", "--dirty"])
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|s| {
                let v = s.trim();
                if v.starts_with('v') {
                    v[1..].to_string()
                } else {
                    v.to_string()
                }
            })
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string())
    };

    println!("cargo:rustc-env=FERRUMCAST_VERSION={}", version);
}