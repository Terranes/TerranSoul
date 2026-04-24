#!/usr/bin/env python3
"""
import_agency_agents.py
=======================
Convert agency-agents .md files into TerranSoul AgentManifest JSON files.

Usage:
    python scripts/import_agency_agents.py --source <path-to-agency-agents-main> [--out agents/]

The script:
1. Scans all .md files in the agency-agents directory tree (skipping README / CONTRIBUTING etc.)
2. Parses YAML frontmatter (name, description, color, emoji, vibe, tools)
3. Extracts the full markdown body as the system prompt
4. Generates one manifest JSON per agent in the output directory
5. Writes a registry.json index listing all generated manifests

Output structure:
    agents/
        registry.json              <- index of all agents
        engineering/
            engineering-frontend-developer.json
            ...
        marketing/
            marketing-content-creator.json
            ...
        ...
"""

import argparse
import json
import os
import re
import sys
from pathlib import Path


# Divisions to process (subfolders of agency-agents repo)
KNOWN_DIVISIONS = [
    "academic",
    "design",
    "engineering",
    "finance",
    "game-development",
    "marketing",
    "paid-media",
    "product",
    "project-management",
    "sales",
    "spatial-computing",
    "specialized",
    "strategy",
    "support",
    "testing",
]

# Files to skip (not agent definitions)
SKIP_FILENAMES = {
    "README.md",
    "CONTRIBUTING.md",
    "CONTRIBUTING_zh-CN.md",
    "SECURITY.md",
    "LICENSE",
    "LICENSE.md",
}


def slugify(name: str) -> str:
    """Convert a human-readable name to a lowercase hyphenated slug."""
    slug = name.lower()
    slug = re.sub(r"[^a-z0-9\s-]", "", slug)
    slug = re.sub(r"[\s_]+", "-", slug)
    slug = re.sub(r"-+", "-", slug)
    slug = slug.strip("-")
    # Truncate to 64 chars as required by TerranSoul manifest spec
    return slug[:64]


def parse_frontmatter(content: str) -> tuple[dict, str]:
    """
    Split YAML frontmatter from markdown body.
    Returns (frontmatter_dict, body_string).
    Frontmatter is delimited by --- lines at the top of the file.
    """
    if not content.startswith("---"):
        return {}, content

    end = content.find("\n---", 3)
    if end == -1:
        return {}, content

    fm_text = content[3:end].strip()
    body = content[end + 4:].strip()

    fm = {}
    for line in fm_text.splitlines():
        if ":" in line:
            key, _, value = line.partition(":")
            fm[key.strip()] = value.strip()

    return fm, body


def make_manifest(division: str, filename: str, fm: dict, body: str) -> dict | None:
    """Build a TerranSoul AgentManifest dict from parsed frontmatter and body."""
    raw_name = fm.get("name", "")
    if not raw_name:
        # Fall back to filename stem
        raw_name = Path(filename).stem.replace("-", " ").replace("_", " ").title()

    description = fm.get("description", "") or fm.get("vibe", "")
    if not description:
        # Extract first non-heading line from body as fallback
        for line in body.splitlines():
            line = line.strip()
            if line and not line.startswith("#"):
                description = line[:200]
                break

    if not description:
        description = f"{raw_name} agent"

    # Build the package name: "<division>-<slugified-name>"
    name_slug = slugify(raw_name)
    package_name = f"{division}-{name_slug}"
    # Ensure it fits within 64 chars
    if len(package_name) > 64:
        package_name = package_name[:64].rstrip("-")

    manifest = {
        "name": package_name,
        "version": "1.0.0",
        "description": description[:500],  # keep reasonable length
        "system_requirements": {
            "min_ram_mb": 0,
            "os": [],
            "arch": [],
            "gpu_required": False,
        },
        "install_method": {
            "type": "system_prompt",
            "content": body,
        },
        "capabilities": ["chat", "conversation_history"],
        "ipc_protocol_version": 1,
        "license": "MIT",
        "author": "agency-agents community (msitarzewski/agency-agents)",
        "homepage": "https://github.com/msitarzewski/agency-agents",
        "division": division,
    }

    # Optional personality fields
    if fm.get("emoji"):
        manifest["emoji"] = fm["emoji"]
    if fm.get("color"):
        manifest["color"] = fm["color"]
    if fm.get("vibe"):
        manifest["vibe"] = fm["vibe"]

    return manifest


def process_division(division_path: Path, division: str, out_dir: Path) -> list[dict]:
    """Process all .md files in a division folder. Returns list of registry entries."""
    entries = []
    out_division = out_dir / division
    out_division.mkdir(parents=True, exist_ok=True)

    md_files = sorted(division_path.glob("*.md"))
    for md_file in md_files:
        if md_file.name in SKIP_FILENAMES:
            continue

        content = md_file.read_text(encoding="utf-8")
        fm, body = parse_frontmatter(content)

        if not body.strip():
            print(f"  SKIP (empty body): {md_file.name}")
            continue

        manifest = make_manifest(division, md_file.name, fm, body)
        if not manifest:
            print(f"  SKIP (no manifest): {md_file.name}")
            continue

        # Write individual manifest JSON
        out_file = out_division / f"{md_file.stem}.json"
        out_file.write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8")

        # Build registry entry (lightweight — no system_prompt content)
        entry = {
            "name": manifest["name"],
            "version": manifest["version"],
            "description": manifest["description"],
            "division": division,
            "manifest_path": str(out_file.relative_to(out_dir)),
            "capabilities": manifest["capabilities"],
        }
        for field in ("emoji", "color", "vibe"):
            if field in manifest:
                entry[field] = manifest[field]

        entries.append(entry)
        print(f"  OK: {manifest['name']}")

    return entries


def main():
    parser = argparse.ArgumentParser(description="Import agency-agents into TerranSoul")
    parser.add_argument(
        "--source",
        default=str(Path(__file__).parent.parent.parent / "agency-agents-main" / "agency-agents-main"),
        help="Path to the agency-agents-main directory (default: sibling of TerranSoul)",
    )
    parser.add_argument(
        "--out",
        default=str(Path(__file__).parent.parent / "agents"),
        help="Output directory for manifest JSON files (default: TerranSoul/agents/)",
    )
    args = parser.parse_args()

    source = Path(args.source)
    out_dir = Path(args.out)

    if not source.exists():
        print(f"ERROR: source directory not found: {source}", file=sys.stderr)
        print("Use --source to specify the path to agency-agents-main/", file=sys.stderr)
        sys.exit(1)

    out_dir.mkdir(parents=True, exist_ok=True)
    print(f"Source : {source}")
    print(f"Output : {out_dir}")
    print()

    all_entries = []

    for division in KNOWN_DIVISIONS:
        division_path = source / division
        if not division_path.exists():
            print(f"[{division}] not found, skipping")
            continue

        print(f"[{division}]")
        entries = process_division(division_path, division, out_dir)
        all_entries.extend(entries)
        print(f"  → {len(entries)} agents\n")

    # Write registry.json
    registry = {
        "version": "1.0.0",
        "source": "msitarzewski/agency-agents",
        "total_agents": len(all_entries),
        "agents": sorted(all_entries, key=lambda e: (e["division"], e["name"])),
    }
    registry_path = out_dir / "registry.json"
    registry_path.write_text(json.dumps(registry, ensure_ascii=False, indent=2), encoding="utf-8")

    print(f"✅ Done: {len(all_entries)} agents imported")
    print(f"   Registry: {registry_path}")


if __name__ == "__main__":
    main()
