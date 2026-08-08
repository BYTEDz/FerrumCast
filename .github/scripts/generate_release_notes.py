#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2025 AZHAR ZOUHIR / BYTEDz

import argparse
import datetime
import json
import re
import subprocess
import sys
import urllib.request
from collections import defaultdict
from typing import Dict, List, Optional, Tuple

CATEGORY_MAPPING = {
    "feat": "Features & Additions",
    "feature": "Features & Additions",
    "add": "Features & Additions",
    "added": "Features & Additions",
    "fix": "Bug Fixes",
    "bugfix": "Bug Fixes",
    "remove": "Removals & Deprecations",
    "removed": "Removals & Deprecations",
    "delete": "Removals & Deprecations",
    "deleted": "Removals & Deprecations",
    "deprecate": "Removals & Deprecations",
    "deprecated": "Removals & Deprecations",
    "drop": "Removals & Deprecations",
    "revert": "Reverts",
    "reverted": "Reverts",
    "sec": "Security Fixes",
    "security": "Security Fixes",
    "perf": "Performance Improvements",
    "refactor": "Refactoring",
    "docs": "Documentation",
    "doc": "Documentation",
    "test": "Tests",
    "tests": "Tests",
    "chore": "Chores & Maintenance",
    "deps": "Chores & Maintenance",
    "build": "Chores & Maintenance",
    "ci": "CI/CD",
    "style": "Style & Formatting",
}

ORDERED_CATEGORIES = [
    "Features & Additions",
    "Bug Fixes",
    "Security Fixes",
    "Removals & Deprecations",
    "Reverts",
    "Performance Improvements",
    "Refactoring",
    "Documentation",
    "Tests",
    "Chores & Maintenance",
    "CI/CD",
    "Style & Formatting",
    "Other Changes",
]

AUTHOR_CACHE: Dict[str, Optional[str]] = {}


def run_git_command(command: List[str]) -> str:
    """Runs a git command and returns the trimmed output."""
    try:
        result = subprocess.run(
            command,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            check=True,
        )
        return result.stdout.strip()
    except subprocess.CalledProcessError as e:
        raise RuntimeError(f"Git command failed: {' '.join(command)}\nError: {e.stderr.strip()}")


def get_latest_tags() -> List[str]:
    """Gets sorted list of tags by creator date (newest first)."""
    try:
        output = run_git_command(["git", "tag", "--sort=-creatordate"])
        return [line.strip() for line in output.split("\n") if line.strip()]
    except Exception:
        return []


def get_remote_url() -> Optional[str]:
    """Attempts to auto-detect the web URL of the remote origin repository."""
    try:
        url = run_git_command(["git", "config", "--get", "remote.origin.url"])
        if not url:
            return None

        if url.endswith(".git"):
            url = url[:-4]

        if url.startswith("git@"):
            parts = url.split(":", 1)
            if len(parts) == 2:
                host = parts[0].replace("git@", "")
                path = parts[1]
                return f"https://{host}/{path}"

        if url.startswith("http://") or url.startswith("https://"):
            return url

        return None
    except Exception:
        return None


def parse_github_repo(repo_url: str) -> Optional[Tuple[str, str]]:
    """Extracts (owner, repo) from a GitHub URL."""
    match = re.search(r"github\.com/([^/]+)/([^/]+)", repo_url)
    if match:
        return match.group(1), match.group(2)
    return None


def fetch_github_username(owner: str, repo: str, commit_hash: str) -> Optional[str]:
    """Queries GitHub API to resolve a commit hash to a GitHub username."""
    url = f"https://api.github.com/repos/{owner}/{repo}/commits/{commit_hash}"
    req = urllib.request.Request(
        url,
        headers={"User-Agent": "Git-Release-Notes-Generator-Python"},
    )
    try:
        with urllib.request.urlopen(req, timeout=3) as response:
            data = json.loads(response.read().decode())
            if data and "author" in data and data["author"] and "login" in data["author"]:
                return data["author"]["login"]
    except Exception:
        pass
    return None


def get_github_username_from_email(email: str) -> Optional[str]:
    """Extracts GitHub username from github noreply email addresses."""
    match = re.match(r"^(?:\d+\+)?([^@]+)@users\.noreply\.github\.com$", email)
    if match:
        return match.group(1)
    return None


def parse_commit_line(line: str) -> Optional[Dict]:
    """Parses a git log line with support for conventional commits and pattern fallbacks."""
    if not line:
        return None
    parts = line.split("|", 3)
    if len(parts) < 4:
        return None

    commit_hash, author, email, subject = parts

    pattern = r"^(?P<type>[a-zA-Z]+)(?:\((?P<scope>[^)]+)\))?(?P<breaking>!)?:\s*(?P<message>.+)$"
    match = re.match(pattern, subject)

    if match:
        gd = match.groupdict()
        return {
            "hash": commit_hash,
            "author": author,
            "email": email,
            "type": gd["type"].lower(),
            "scope": gd["scope"],
            "breaking": bool(gd["breaking"]),
            "message": gd["message"],
            "raw_subject": subject,
        }

    lower_subj = subject.strip().lower()
    is_breaking = "breaking change" in lower_subj or "breaking:" in lower_subj
    detected_type = "other"

    if re.match(r"^(add|added|new|create|created|feat|feature)\b", lower_subj):
        detected_type = "feat"
    elif re.match(r"^(fix|fixed|bugfix|resolv|resolve|resolved)\b", lower_subj):
        detected_type = "fix"
    elif re.match(r"^(remove|removed|delete|deleted|deprecate|deprecated|drop|dropped)\b", lower_subj):
        detected_type = "remove"
    elif re.match(r"^(revert|reverted)\b", lower_subj):
        detected_type = "revert"
    elif re.match(r"^(sec|security|vulnerability)\b", lower_subj):
        detected_type = "sec"
    elif re.match(r"^(perf|optimize|performance)\b", lower_subj):
        detected_type = "perf"
    elif re.match(r"^(refactor|clean|cleanup)\b", lower_subj):
        detected_type = "refactor"
    elif re.match(r"^(doc|docs|readme)\b", lower_subj):
        detected_type = "docs"
    elif re.match(r"^(test|tests|spec|specs)\b", lower_subj):
        detected_type = "test"
    elif re.match(r"^(chore|deps|build|ci)\b", lower_subj):
        detected_type = "chore"
    elif re.match(r"^(style|fmt|format)\b", lower_subj):
        detected_type = "style"

    return {
        "hash": commit_hash,
        "author": author,
        "email": email,
        "type": detected_type,
        "scope": None,
        "breaking": is_breaking,
        "message": subject,
        "raw_subject": subject,
    }


def format_commit_entry(commit: Dict, repo_url: Optional[str]) -> str:
    """Formats a commit entry matching: <subject> by <author> in <PR/hash>."""
    author_display = f"@{commit['author']}"

    if repo_url and "github.com" in repo_url:
        git_author = commit["author"]

        if git_author in AUTHOR_CACHE:
            github_username = AUTHOR_CACHE[git_author]
        else:
            github_username = get_github_username_from_email(commit["email"])
            if not github_username:
                repo_info = parse_github_repo(repo_url)
                if repo_info:
                    owner, repo = repo_info
                    github_username = fetch_github_username(owner, repo, commit["hash"])

            AUTHOR_CACHE[git_author] = github_username

        if github_username:
            author_display = f"[@{github_username}](https://github.com/{github_username})"

    raw_subject = commit["raw_subject"]
    pr_match = re.search(r"\s*\(#(\d+)\)$", raw_subject)

    if pr_match:
        pr_num = pr_match.group(1)
        clean_subject = raw_subject[:pr_match.start()].strip()
        if repo_url:
            ref_str = f"in [#{pr_num}]({repo_url}/pull/{pr_num})"
        else:
            ref_str = f"in #{pr_num}"
    else:
        clean_subject = raw_subject
        hash_str = commit["hash"]
        if repo_url:
            ref_str = f"in [{hash_str}]({repo_url}/commit/{hash_str})"
        else:
            ref_str = f"in {hash_str}"

    return f"- {clean_subject} by {author_display} {ref_str}"


def generate_changelog(from_ref: Optional[str], to_ref: str) -> str:
    """Generates structured release notes from a range of commits or entire history if first release."""
    log_format = "%h|%an|%ae|%s"
    if from_ref:
        log_range = f"{from_ref}..{to_ref}"
    else:
        log_range = to_ref

    try:
        log_output = run_git_command(["git", "log", f"--format={log_format}", log_range])
    except RuntimeError as e:
        sys.exit(f"Error reading git log for range {log_range}: {e}")

    commits = [parse_commit_line(line) for line in log_output.split("\n") if line.strip()]
    commits = [c for c in commits if c is not None]

    if not commits:
        return "No changes found."

    repo_url = get_remote_url()
    other_cat = "Other Changes"

    grouped: Dict[str, List[Dict]] = defaultdict(list)
    breaking_changes: List[Dict] = []

    for commit in commits:
        if commit["raw_subject"].startswith("Merge branch") or commit["raw_subject"].startswith("Merge pull request"):
            continue

        if commit["breaking"]:
            breaking_changes.append(commit)

        category = CATEGORY_MAPPING.get(commit["type"], other_cat)
        grouped[category].append(commit)

    today = datetime.date.today().isoformat()
    version_display = to_ref if to_ref != "HEAD" else "v0.1.0"

    output = []
    output.append(f"# Release Notes - {version_display} ({today})")
    output.append("")

    if breaking_changes:
        output.append("## BREAKING CHANGES")
        for bc in breaking_changes:
            output.append(format_commit_entry(bc, repo_url))
        output.append("")

    for cat in ORDERED_CATEGORIES:
        if cat in grouped and grouped[cat]:
            output.append(f"## {cat}")
            for commit in grouped[cat]:
                output.append(format_commit_entry(commit, repo_url))
            output.append("")

    return "\n".join(output)


def main():
    parser = argparse.ArgumentParser(description="Generate structured release notes from Git Diff.")
    parser.add_argument("--from", "-f", dest="from_ref", help="Start ref. Auto-detects latest tag if empty.")
    parser.add_argument("--to", "-t", dest="to_ref", default="HEAD", help="End ref (default: HEAD).")
    parser.add_argument("--output", "-o", default="RELEASE_NOTES.md", help="File path to save release notes.")

    args = parser.parse_args()

    try:
        run_git_command(["git", "rev-parse", "--is-inside-work-tree"])
    except RuntimeError:
        sys.exit("Error: Not inside a git repository.")

    from_ref = args.from_ref
    to_ref = args.to_ref

    if not from_ref:
        tags = get_latest_tags()
        head_commit = run_git_command(["git", "rev-parse", "HEAD"])

        # Filter out any tag pointing to current HEAD (the tag currently being published)
        previous_tags = []
        for tag in tags:
            tag_commit = run_git_command(["git", "rev-parse", f"{tag}^{{commit}}"])
            if tag_commit != head_commit:
                previous_tags.append(tag)

        if previous_tags:
            from_ref = previous_tags[0]
        else:
            from_ref = None

    changelog = generate_changelog(from_ref, to_ref)

    if args.output:
        try:
            with open(args.output, "w", encoding="utf-8") as f:
                f.write(changelog)
            print(f"Success: Release notes written to {args.output}", file=sys.stderr)
        except IOError as e:
            sys.exit(f"Error writing to file: {e}")
    else:
        print(changelog)


if __name__ == "__main__":
    main()