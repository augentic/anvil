#!/usr/bin/env bash

set -euo pipefail
SOURCE_BASE="$HOME/rust/github.com/augentic/specify/plugins"

mkdir -p .cursor/skills
for skill_dir in "$SOURCE_BASE"/*/skills/*/; do
    [ -d "$skill_dir" ] || continue
    skill_name="$(basename "$skill_dir")"
    link=".cursor/skills/$skill_name"
    if [ -L "$link" ]; then
        echo "skip (exists): $skill_name"
    elif [ -e "$link" ]; then
        echo "skip (non-symlink exists): $skill_name"
    else
        ln -s "$skill_dir" "$link"
        echo "linked: $skill_name -> $skill_dir"
    fi
done