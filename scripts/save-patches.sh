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
  # new files are not tracked yet: mark them so that the diff includes them
  git -C "$UP" add -N -- "$@" 2>/dev/null || true
  # keep the explanation that precedes the diff
  if [ -f "$out" ]; then head="$(sed '/^diff --git/,$d' "$out")"; fi
  { [ -n "$head" ] && printf '%s\n' "$head"; git -C "$UP" diff -- "$@"; } > "$out.new"
  mv "$out.new" "$out"
  echo "saved $out ($(git -C "$UP" diff --stat -- "$@" | tail -1))"
}

save 0001-mobile-layout-hook crates/ui-egui/src/lib.rs crates/ui-egui/src/panels/edit.rs
save 0002-touch-friendly-widgets crates/ui-egui/src/widgets.rs
save 0003-light-and-noise-reduction crates/pipeline/src/auto.rs crates/pipeline/src/nr.rs crates/pipeline/src/local.rs crates/pipeline/src/lib.rs crates/gpu/src/render.rs
save 0004-lens-profiles crates/develop/src/settings.rs crates/pipeline/src/optics.rs crates/engine/src/lens_profiles.rs crates/engine/src/cmd/lens.rs crates/engine/src/cmd/mod.rs crates/engine/src/media.rs
save 0005-touch-selectors crates/ui-egui/src/panels/detail.rs crates/ui-egui/src/panels/masking.rs
