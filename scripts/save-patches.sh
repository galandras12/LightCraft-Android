#!/usr/bin/env bash
# Writes the changes made inside third_party/lightcraft back into patches/*.patch.
#
# third_party/lightcraft is reset to the pinned upstream revision by scripts/fetch-upstream.sh, so
# an edit there is lost unless it is saved as a patch first. Each patch covers one topic (the files
# listed below); the descriptive text above the first "diff --git" line is kept from the old patch.
set -euo pipefail
cd "$(dirname "$0")/.."
UP=third_party/lightcraft

# save <patch name> <file>...   (files are relative to the upstream checkout)
save() {
  local name="$1"; shift
  local out="patches/$name.patch"
  local head=""
  # keep the explanation that precedes the diff
  if [ -f "$out" ]; then head="$(sed '/^diff --git/,$d' "$out")"; fi
  { [ -n "$head" ] && printf '%s\n' "$head"; git -C "$UP" diff -- "$@"; } > "$out.new"
  mv "$out.new" "$out"
  echo "saved $out ($(git -C "$UP" diff --stat -- "$@" | tail -1))"
}

save 0001-mobile-layout-hook crates/ui-egui/src/lib.rs crates/ui-egui/src/panels/edit.rs
save 0002-touch-friendly-widgets crates/ui-egui/src/widgets.rs
