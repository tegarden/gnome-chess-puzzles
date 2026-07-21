#!/bin/sh
set -eu

application_id=io.github.tegarden.gnome-chess-puzzles
manifest="$application_id.yml"
bundle="dist/$application_id.flatpak"
remote=${GCP_FLATPAK_REMOTE:-flathub}
dependency_option=

if command -v flatpak-builder >/dev/null 2>&1; then
  builder=flatpak-builder
elif flatpak info org.flatpak.Builder >/dev/null 2>&1; then
  builder="flatpak run --env=FLATPAK_USER_DIR=$HOME/.local/share/flatpak --command=flatpak-builder org.flatpak.Builder"
else
  echo "flatpak-builder or the org.flatpak.Builder Flatpak is required" >&2
  exit 1
fi

if ! flatpak info org.gnome.Sdk//50 >/dev/null 2>&1 ||
  ! flatpak info org.freedesktop.Sdk.Extension.rust-stable//25.08 >/dev/null 2>&1; then
  dependency_option="--install-deps-from=$remote"
fi

mkdir -p dist
$builder \
  --force-clean \
  $dependency_option \
  --repo=flatpak-repo \
  flatpak-build \
  "$manifest"
flatpak build-bundle \
  flatpak-repo \
  "$bundle" \
  "$application_id" \
  --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo

echo "Created $bundle"
