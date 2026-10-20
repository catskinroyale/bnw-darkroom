#!/bin/bash
# Cut a release with semantic versioning.
#
#   tools/release.sh patch|minor|major [--dry-run]
#
# 1. Bumps the workspace version in Cargo.toml (the single source of truth: the app,
#    the About window, export recipes and the macOS bundle all read it).
# 2. Moves CHANGELOG.md's [Unreleased] entries into a dated [X.Y.Z] section and
#    updates the comparison links.
# 3. Commits "Release vX.Y.Z" and creates the annotated tag vX.Y.Z.
#
# It does not push. Afterwards: git push && git push origin vX.Y.Z
#
# Before 1.0.0: minor = new features or breaking changes, patch = fixes.
set -euo pipefail
cd "$(dirname "$0")/.."

BUMP="${1:-}"
DRY_RUN="${2:-}"
case "$BUMP" in patch|minor|major) ;; *) echo "usage: $0 patch|minor|major [--dry-run]" >&2; exit 2 ;; esac

if [ "$DRY_RUN" != "--dry-run" ] && [ -n "$(git status --porcelain)" ]; then
  echo "error: working tree has uncommitted changes; commit them first" >&2
  exit 1
fi

python3 -I - "$BUMP" "$DRY_RUN" <<'PY'
import datetime, re, subprocess, sys
bump, dry = sys.argv[1], sys.argv[2] == "--dry-run"

cargo = open("Cargo.toml").read()
m = re.search(r'(\[workspace\.package\][^\[]*?\nversion = ")(\d+)\.(\d+)\.(\d+)(")', cargo, re.S)
if not m:
    sys.exit("error: no version in [workspace.package]")
major, minor, patch = map(int, m.group(2, 3, 4))
old = f"{major}.{minor}.{patch}"
if bump == "major": major, minor, patch = major + 1, 0, 0
elif bump == "minor": minor, patch = minor + 1, 0
else: patch += 1
new = f"{major}.{minor}.{patch}"

log = open("CHANGELOG.md").read()
section = re.search(r"## \[Unreleased\]\n(.*?)(?=\n## \[)", log, re.S)
if not section or not section.group(1).strip():
    sys.exit("error: CHANGELOG.md has nothing under [Unreleased]")
today = datetime.date.today().isoformat()
log = log.replace("## [Unreleased]\n", f"## [Unreleased]\n\n## [{new}] - {today}\n", 1)

# Comparison links: [Unreleased] now starts at the new tag; the new version runs from
# the previous tag (or the commit the previous link pointed at, before tags existed).
repo = "https://github.com/catskinroyale/bnw-darkroom"
has_old_tag = subprocess.run(["git", "rev-parse", "-q", "--verify", f"refs/tags/v{old}"],
                             capture_output=True).returncode == 0
prev = f"v{old}"
if not has_old_tag:
    link = re.search(rf"^\[{re.escape(old)}\]: \S*?/(?:commit/|compare/\S*?\.\.\.)(\S+)$", log, re.M)
    prev = link.group(1) if link else prev
log = re.sub(r"^\[Unreleased\]: .*$", f"[Unreleased]: {repo}/compare/v{new}...HEAD\n"
             f"[{new}]: {repo}/compare/{prev}...v{new}", log, count=1, flags=re.M)

cargo = cargo[:m.start()] + f"{m.group(1)}{new}{m.group(5)}" + cargo[m.end():]
print(f"{old} -> {new} ({bump}); changes since {prev}:")
print(section.group(1).strip())
if dry:
    print("\n--dry-run: nothing written")
    sys.exit(0)
open("Cargo.toml", "w").write(cargo)
open("CHANGELOG.md", "w").write(log)
open(".release-version", "w").write(new)
PY

[ "$DRY_RUN" = "--dry-run" ] && exit 0
NEW=$(cat .release-version) && rm .release-version
cargo update --workspace --quiet          # refresh workspace versions in Cargo.lock
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit --quiet -m "Release v$NEW"
NOTES=$(sed -n "/^## \[$NEW\]/,/^## \[/p" CHANGELOG.md | sed '1d;$d')
git tag -a --cleanup=whitespace "v$NEW" -m "BW Darkroom $NEW" -m "$NOTES"
echo "Tagged v$NEW. Push with: git push && git push origin v$NEW"
