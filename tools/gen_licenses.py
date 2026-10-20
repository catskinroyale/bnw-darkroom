#!/usr/bin/env python3
"""List the third-party crates compiled into the desktop app, with their licences.

Writes apps/bw-desktop/assets/third-party-licenses.json, which the About dialog shows.
Only normal (shipped) dependencies of bw-desktop for the packaging target are listed;
build-time and test-only crates are not part of the app. Re-run after changing
dependencies (tools/package-macos.sh runs it automatically).

Usage: python3 -I tools/gen_licenses.py [target-triple]   (default aarch64-apple-darwin)
"""
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "apps/bw-desktop/assets/third-party-licenses.json"


def main() -> int:
    target = sys.argv[1] if len(sys.argv) > 1 else "aarch64-apple-darwin"
    meta = json.loads(subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--filter-platform", target],
        cwd=ROOT, check=True, capture_output=True, text=True).stdout)
    packages = {p["id"]: p for p in meta["packages"]}
    nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
    workspace = set(meta["workspace_members"])
    root = next(p["id"] for p in meta["packages"] if p["name"] == "bw-desktop")

    seen, stack = set(), [root]
    while stack:
        pid = stack.pop()
        if pid in seen:
            continue
        seen.add(pid)
        for dep in nodes[pid]["deps"]:
            if any(k["kind"] is None for k in dep["dep_kinds"]):  # normal dependency
                stack.append(dep["pkg"])

    rows = sorted(
        ({
            "name": packages[p]["name"],
            "version": packages[p]["version"],
            "license": packages[p].get("license") or "see repository",
            "repository": packages[p].get("repository") or "",
        } for p in seen if p not in workspace),
        key=lambda r: (r["name"].lower(), r["version"]),
    )
    OUT.write_text(json.dumps({"target": target, "crates": rows}, indent=1) + "\n")
    print(f"{len(rows)} third-party crates -> {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
