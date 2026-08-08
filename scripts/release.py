#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2025 AZHAR ZOUHIR / BYTEDz

import datetime
import re
import subprocess
import sys
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parent.parent
CARGO_TOML = ROOT_DIR / "Cargo.toml"
CHANGELOG_FILE = ROOT_DIR / "CHANGELOG.md"


def run_cmd(cmd: list[str], check: bool = True) -> str:
    """Executes a subprocess command in the repository root directory."""
    result = subprocess.run(cmd, capture_output=True, text=True, cwd=ROOT_DIR)
    if check and result.returncode != 0:
        error_msg = result.stderr.strip() or result.stdout.strip()
        print(f"Error executing: {' '.join(cmd)}\n{error_msg}")
        sys.exit(1)
    return result.stdout.strip()


def generate_recent_commit_summary(version: str) -> str:
    """Extracts commits since the previous tag or initial commit."""
    tags = run_cmd(["git", "tag", "--sort=-creatordate"], check=False).splitlines()
    prev_ref = tags[0] if tags else run_cmd(["git", "rev-list", "--max-parents=0", "HEAD"])

    log_output = run_cmd(["git", "log", f"{prev_ref}..HEAD", "--oneline"], check=False)
    if not log_output:
        return "- Maintenance and internal updates."

    lines = []
    for line in log_output.splitlines():
        # Strip commit hash
        parts = line.strip().split(" ", 1)
        if len(parts) == 2:
            lines.append(f"- {parts[1]}")
    return "\n".join(lines)


def update_changelog(version: str):
    """Prepends new version release notes to CHANGELOG.md."""
    today = datetime.date.today().isoformat()
    section_header = f"## [{version}] - {today}\n"
    notes = generate_recent_commit_summary(version)
    new_entry = f"{section_header}\n{notes}\n"

    main_header = "# Changelog\n\nAll notable changes to this project will be documented in this file.\n\n"

    if CHANGELOG_FILE.exists():
        existing_content = CHANGELOG_FILE.read_text(encoding="utf-8")
        if existing_content.startswith("# Changelog"):
            existing_content = existing_content.replace(main_header, "", 1).lstrip()
        updated_content = f"{main_header}{new_entry}\n{existing_content}"
    else:
        updated_content = f"{main_header}{new_entry}"

    CHANGELOG_FILE.write_text(updated_content, encoding="utf-8")


def main():
    if len(sys.argv) < 2:
        print("Error: Version argument missing.")
        print("Usage: python3 scripts/release.py <version>  (e.g., python3 scripts/release.py 0.2.0)")
        sys.exit(1)

    version = sys.argv[1].lstrip("v")
    tag = f"v{version}"

    if not CARGO_TOML.exists():
        print(f"Error: Could not locate Cargo.toml at {CARGO_TOML}")
        sys.exit(1)

    # Verify working tree is clean
    git_status = run_cmd(["git", "status", "--porcelain"])
    if git_status:
        print("Error: Working directory has uncommitted changes. Commit or stash them first.")
        sys.exit(1)

    # Verify current branch is main
    current_branch = run_cmd(["git", "rev-parse", "--abbrev-ref", "HEAD"])
    if current_branch != "main":
        print(f"Error: Releases must be initiated from 'main' (currently on '{current_branch}').")
        sys.exit(1)

    print(f"--> Bumping package version to {version} in Cargo.toml...")
    cargo_content = CARGO_TOML.read_text(encoding="utf-8")
    updated_content = re.sub(
        r'^version\s*=\s*"[^"]+"',
        f'version = "{version}"',
        cargo_content,
        count=1,
        flags=re.MULTILINE,
    )
    CARGO_TOML.write_text(updated_content, encoding="utf-8")

    print("--> Updating CHANGELOG.md...")
    update_changelog(version)

    print("--> Validating Cargo.toml syntax...")
    run_cmd(["cargo", "check", "--quiet"])

    print(f"--> Committing release {tag}...")
    run_cmd(["git", "add", "Cargo.toml", "CHANGELOG.md"])
    run_cmd(["git", "commit", "-m", f"chore(release): {tag}"])

    print(f"--> Creating annotated tag {tag}...")
    run_cmd(["git", "tag", "-a", tag, "-m", f"Release {tag}"])

    print(f"--> Pushing main branch and tag {tag} to origin...")
    run_cmd(["git", "push", "origin", "main"])
    run_cmd(["git", "push", "origin", tag])

    print("==================================================")
    print(f"Release {tag} published successfully.")
    print("Cargo.toml and CHANGELOG.md have been updated and pushed.")
    print("==================================================")


if __name__ == "__main__":
    main()