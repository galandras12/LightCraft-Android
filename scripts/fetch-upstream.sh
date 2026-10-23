#!/usr/bin/env bash
# Checks out the pinned upstream LightCraft (https://github.com/storytold/lightcraft) into
# third_party/lightcraft and applies every patches/*.patch to it. Safe to run again: it resets the
# checkout to the pinned revision first.
set -euo pipefail
cd "$(dirname "$0")/.."
REV=1c03b7b18f3a8219b034ad384477bf2f8d767585
DIR=third_party/lightcraft
if [ ! -d "$DIR/.git" ]; then
  mkdir -p "$DIR" && git -C "$DIR" init -q
  git -C "$DIR" remote add origin https://github.com/storytold/lightcraft
fi
if [ "$(git -C "$DIR" rev-parse HEAD 2>/dev/null || true)" != "$REV" ]; then
  git -C "$DIR" fetch -q --depth 1 origin "$REV"
  git -C "$DIR" checkout -q -f "$REV"
fi
# back to the pinned revision, dropping earlier edits and the files earlier patches added
git -C "$DIR" reset -q --hard "$REV"
git -C "$DIR" clean -fdq
for p in patches/*.patch; do git -C "$DIR" apply "../../$p"; done
echo "upstream $REV patched in $DIR"
