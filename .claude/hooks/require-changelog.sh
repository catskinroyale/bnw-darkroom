#!/bin/bash
# PreToolUse hook (Bash): block `git push` unless CHANGELOG.md is part of what is being
# pushed. It counts as updated if it changed in the unpushed commits, or is staged or
# modified now (covers `git commit ... && git push` in one command, where the hook runs
# before the commit exists).

input=$(cat)
command=$(printf '%s' "$input" | jq -r '.tool_input.command // empty')
case "$command" in
  *"git push"*) ;;
  *) exit 0 ;;
esac

cwd=$(printf '%s' "$input" | jq -r '.cwd // empty')
[ -n "$cwd" ] && cd "$cwd" 2>/dev/null
git rev-parse --git-dir >/dev/null 2>&1 || exit 0

base=$(git rev-parse --verify -q '@{u}' 2>/dev/null || git rev-parse --verify -q origin/main 2>/dev/null)
[ -z "$base" ] && exit 0

changed=$( {
  git diff --name-only "$base"..HEAD
  git diff --name-only --cached
  git diff --name-only
} 2>/dev/null )

if printf '%s\n' "$changed" | grep -qx 'CHANGELOG.md'; then
  exit 0
fi

# Nothing new to push and nothing pending: let it through.
if [ -z "$(git rev-list "$base"..HEAD 2>/dev/null)" ] && [ -z "$(git status --porcelain 2>/dev/null)" ]; then
  exit 0
fi

jq -n '{
  hookSpecificOutput: {
    hookEventName: "PreToolUse",
    permissionDecision: "deny",
    permissionDecisionReason: "Project rule: update CHANGELOG.md (Unreleased section) to describe the changes being pushed, commit it, then push again."
  }
}'
