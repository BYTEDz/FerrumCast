#!/usr/bin/env python3
import argparse
import json
import re
import subprocess
import sys
import urllib.request
from collections import defaultdict
from typing import Dict, List, Optional, Tuple

CATEGORY_MAPPING_CLEAN = {
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

CATEGORY_MAPPING_EMOJI = {
    "feat": "🚀 Features & Additions",
    "feature": "🚀 Features & Additions",
    "add": "🚀 Features & Additions",
    "added": "🚀 Features & Additions",
    "fix": "🐛 Bug Fixes",
    "bugfix": "🐛 Bug Fixes",
    "remove": "🗑️ Removals & Deprecations",
    "removed": "🗑️ Removals & Deprecations",
    "delete": "🗑️ Removals & Deprecations",
    "deleted": "🗑️ Removals & Deprecations",
    "deprecate": "🗑️ Removals & Deprecations",
    "deprecated": "🗑️ Removals & Deprecations",
    "drop": "🗑️ Removals & Deprecations",
    "revert": "⏪ Reverts",
    "reverted": "⏪ Reverts",
    "sec": "🔒 Security Fixes",
    "security": "🔒 Security Fixes",
    "perf": "⚡ Performance Improvements",
    "refactor": "🛠️ Refactoring",
    "docs": "📝 Documentation",
    "doc": "📝 Documentation",
    "test": "🧪 Tests",
    "tests": "🧪 Tests",
    "chore": "🧹 Chores & Maintenance",
    "deps": "🧹 Chores & Maintenance",
    "build": "🧹 Chores & Maintenance",
    "ci": "⚙️ CI/CD",
    "style": "🎨 Style & Formatting",
}

ORDERED_CATEGORIES_CLEAN = [
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

ORDERED_CATEGORIES_EMOJI = [
    "🚀 Features & Additions",
    "🐛 Bug Fixes",
    "🔒 Security Fixes",
    "🗑️ Removals & Deprecations",
    "⏪ Reverts",
    "⚡ Performance Improvements",
    "🛠️ Refactoring",
    "📝 Documentation",
    "🧪 Tests",
    "🧹 Chores & Maintenance",
    "⚙️ CI/CD",
    "🎨 Style & Formatting",
    "🔄 Other Changes",
]

# Cache to map Git Author Name -> GitHub Username
# This prevents duplicate API calls for the same author
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


def get_initial_commit() -> str:
    """Gets the first commit hash of the repository."""
    return run_git_command(["git", "rev-list", "--max-parents=0", "HEAD"])


def get_remote_url() -> Optional[str]:
    """Attempts to auto-detect the web URL of the remote origin repository."""
    try:
        url = run_git_command(["git", "config", "--get", "remote.origin.url"])
        if not url:
            return None
        
        if url.endswith(".git"):
            url = url[:-4]
            
        # Convert SSH style (git@github.com:user/repo) to HTTPS URL
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
        headers={"User-Agent": "Git-Release-Notes-Generator-Python"}
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
    
    # 1. Conventional Commit regex pattern: type(scope)!: message
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
            "raw_subject": subject
        }

    # 2. Fallback heuristic detection for non-conventional commit messages
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
        "raw_subject": subject
    }


def format_commit_entry(commit: Dict, repo_url: Optional[str]) -> str:
    """Formats a single commit line with clickable links."""
    author_display = f"@{commit['author']}"
    
    if repo_url and "github.com" in repo_url:
        git_author = commit["author"]
        
        # Check cache first
        if git_author in AUTHOR_CACHE:
            github_username = AUTHOR_CACHE[git_author]
        else:
            # Try parsing from standard GitHub noreply email address
            github_username = get_github_username_from_email(commit["email"])
            
            # If not a noreply email, fetch directly via API using the commit hash
            if not github_username:
                repo_info = parse_github_repo(repo_url)
                if repo_info:
                    owner, repo = repo_info
                    github_username = fetch_github_username(owner, repo, commit["hash"])
            
            # Store result in cache (even if None, to avoid duplicate API spamming)
            AUTHOR_CACHE[git_author] = github_username
        
        if github_username:
            author_display = f"[@{github_username}](https://github.com/{github_username})"

    # Format commit hash link
    hash_str = commit["hash"]
    if repo_url:
        hash_str = f"[{hash_str}]({repo_url}/commit/{hash_str})"
    
    # Format Pull Request link if present
    message = commit["message"]
    if repo_url:
        pr_match = re.search(r"\(#(\d+)\)$", message)
        if pr_match:
            pr_num = pr_match.group(1)
            message = message[:pr_match.start()].strip()
            message = f"{message} ([#{pr_num}]({repo_url}/pull/{pr_num}))"

    scope_str = f"**{commit['scope']}**: " if commit['scope'] else ""
    return f"- {scope_str}{message} ({hash_str}) by {author_display}"


def generate_changelog(from_ref: str, to_ref: str, use_emojis: bool) -> str:
    """Generates structured release notes from a range of commits."""
    log_format = "%h|%an|%ae|%s"
    log_range = f"{from_ref}..{to_ref}"
    
    try:
        log_output = run_git_command(["git", "log", f"--format={log_format}", log_range])
    except RuntimeError as e:
        sys.exit(f"Error reading git log for range {log_range}: {e}")

    commits = [parse_commit_line(line) for line in log_output.split("\n") if line.strip()]
    commits = [c for c in commits if c is not None]

    if not commits:
        return f"No changes found between `{from_ref}` and `{to_ref}`."

    repo_url = get_remote_url()
    category_map = CATEGORY_MAPPING_EMOJI if use_emojis else CATEGORY_MAPPING_CLEAN
    other_cat = "🔄 Other Changes" if use_emojis else "Other Changes"

    grouped: Dict[str, List[Dict]] = defaultdict(list)
    breaking_changes: List[Dict] = []

    for commit in commits:
        if commit["raw_subject"].startswith("Merge branch") or commit["raw_subject"].startswith("Merge pull request"):
            continue

        if commit["breaking"]:
            breaking_changes.append(commit)
        
        category = category_map.get(commit["type"], other_cat)
        grouped[category].append(commit)

    output = []
    output.append(f"# Release Notes ({from_ref} -> {to_ref})")
    output.append("")

    if breaking_changes:
        header = "🚨 BREAKING CHANGES" if use_emojis else "BREAKING CHANGES"
        output.append(f"## {header}")
        for bc in breaking_changes:
            output.append(format_commit_entry(bc, repo_url))
        output.append("")

    ordered_categories = ORDERED_CATEGORIES_EMOJI if use_emojis else ORDERED_CATEGORIES_CLEAN

    for cat in ordered_categories:
        if cat in grouped and grouped[cat]:
            output.append(f"## {cat}")
            for commit in grouped[cat]:
                output.append(format_commit_entry(commit, repo_url))
            output.append("")

    return "\n".join(output)


def main():
    parser = argparse.ArgumentParser(description="Generate structured release notes from Git Diff.")
    parser.add_argument("--from", "-f", dest="from_ref", help="Start tag, commit, or branch. Auto-detects the latest tag if empty.")
    parser.add_argument("--to", "-t", dest="to_ref", default="HEAD", help="End tag, commit, or branch (default: HEAD).")
    parser.add_argument("--output", "-o", default="RELEASE_NOTES.md", help="File path to save release notes (default: RELEASE_NOTES.md).")
    parser.add_argument("--emoji", action="store_true", help="Add emojis to headings (disabled by default).")

    args = parser.parse_args()

    try:
        run_git_command(["git", "rev-parse", "--is-inside-work-tree"])
    except RuntimeError:
        sys.exit("Error: Not inside a git repository.")

    from_ref = args.from_ref
    to_ref = args.to_ref

    if not from_ref:
        tags = get_latest_tags()
        if tags:
            if to_ref == "HEAD":
                from_ref = tags[0]
            else:
                try:
                    to_index = tags.index(to_ref)
                    if to_index + 1 < len(tags):
                        from_ref = tags[to_index + 1]
                    else:
                        from_ref = get_initial_commit()
                except ValueError:
                    from_ref = tags[0]
        else:
            try:
                from_ref = get_initial_commit()
            except Exception:
                sys.exit("Error: Could not retrieve git repository history.")

    print(f"Generating changelog from '{from_ref}' to '{to_ref}'...", file=sys.stderr)
    
    changelog = generate_changelog(from_ref, to_ref, args.emoji)

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