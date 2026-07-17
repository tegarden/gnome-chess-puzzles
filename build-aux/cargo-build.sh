#!/bin/sh
set -eu

source_dir=$1
target_dir=$2
build_type=$3
output=$4

case "$build_type" in
  release|minsize)
    profile=release
    release_flag=--release
    ;;
  *)
    profile=debug
    release_flag=
    ;;
esac

CARGO_TARGET_DIR="$target_dir" cargo build \
  --manifest-path "$source_dir/Cargo.toml" $release_flag
cp "$target_dir/$profile/gnome-chess-puzzles" "$output"

