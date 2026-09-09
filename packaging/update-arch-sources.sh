#!/usr/bin/env bash
# Regenerate local source checksums after changing Rust or packaged assets.
set -euo pipefail
cd "$(dirname "$0")/.."
mapfile -t sources < <(find rust/src -type f -name '*.rs' | LC_ALL=C sort)
sources=(rust/Cargo.toml rust/Cargo.lock rust/build.rs "${sources[@]}" ai-dikte.png ai-dikte.ico ai-dikte-toggle ai-dikte.desktop ai-dikte-settings.desktop packaging/install-linux.sh packaging/linux/ai-dikte.service LICENSE)
sed '/^# BEGIN GENERATED SOURCES/,$d' PKGBUILD > PKGBUILD.tmp
{
  echo '# BEGIN GENERATED SOURCES'
  echo '_sources=('
  printf "  '%s'\n" "${sources[@]}"
  echo ')'
  cat <<'SOURCES'
# makepkg resolves local sources by basename. Explicit file URLs retain the
# checkout path for files in subdirectories while checksums remain mandatory.
DLAGENTS+=('file::/usr/bin/curl -qg -o %o %u')
source=()
for file in "${_sources[@]}"; do
  source+=("${file##*/}::file://$startdir/$file")
done
SOURCES
  echo 'sha256sums=('
  for file in "${sources[@]}"; do
    sum=$(sha256sum "$file")
    printf "  '%s'\n" "${sum%% *}"
  done
  echo ')'
  cat <<'BUILD'

prepare() {
  for file in "${_sources[@]}"; do
    install -Dm644 "$srcdir/${file##*/}" "$srcdir/project/$file"
  done
  cd "$srcdir/project/rust"
  cargo fetch --locked
}
build() {
  cd "$srcdir/project/rust"
  cargo build --frozen --release
}
check() {
  cd "$srcdir/project/rust"
  cargo test --frozen --release
  ./target/release/ai-dikte --self-test
}
package() {
  cd "$srcdir/project"
  DESTDIR="$pkgdir" AI_DIKTE_BINARY=rust/target/release/ai-dikte sh packaging/install-linux.sh
}
BUILD
} >> PKGBUILD.tmp
mv PKGBUILD.tmp PKGBUILD
