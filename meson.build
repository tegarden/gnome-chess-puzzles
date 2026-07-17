project(
  'gnome-chess-puzzles',
  'rust',
  version: '0.1.0',
  license: 'GPL-3.0-or-later',
  meson_version: '>= 1.3.0',
)

dependency('gtk4', version: '>= 4.6')
dependency('libadwaita-1', version: '>= 1.8')

cargo = find_program('cargo')
cargo_build = find_program('build-aux/cargo-build.sh')

custom_target(
  'gnome-chess-puzzles',
  output: 'gnome-chess-puzzles',
  command: [
    cargo_build,
    meson.project_source_root(),
    meson.project_build_root() / 'target',
    get_option('buildtype'),
    '@OUTPUT@',
  ],
  build_by_default: true,
  build_always_stale: true,
  install: true,
  install_dir: get_option('bindir'),
)

install_data(
  'data/io.github.tegarden.GnomeChessPuzzles.desktop',
  install_dir: get_option('datadir') / 'applications',
)
