#!/bin/sh
set -eu

source_dir=$1
target_dir=$2
build_type=$3
output=$4
data_dir=$5

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

GCP_DATA_DIR="$data_dir" CARGO_TARGET_DIR="$target_dir" cargo build \
  --manifest-path "$source_dir/Cargo.toml" $release_flag
cp "$target_dir/$profile/gnome-chess-puzzles" "$output"
