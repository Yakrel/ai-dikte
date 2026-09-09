#!/usr/bin/env bash
# Regenerate local source checksums after changing Rust or packaged assets.
set -euo pipefail
cd "$(dirname "$0")/.."
mapfile -t sources < <(find rust/src -type f -name '*.rs' | LC_ALL=C sort)
sources=(rust/Cargo.toml rust/Cargo.lock rust/build.rs "${sources[@]}" ai-dikte.png ai-dikte.ico ai-dikte-toggle ai-dikte.desktop ai-dikte-settings.desktop packaging/install-linux.sh packaging/linux/ai-dikte.service LICENSE)
sed '/^# BEGIN GENERATED SOURCES/,$d' PKGBUILD > PKGBUILD.tmp
{
  echo '# BEGIN GENERATED SOURCES'
  echo 'source=('
  printf "  '%s'\n" "${sources[@]}"
  echo ')'
  echo 'sha256sums=('
  for file in "${sources[@]}"; do
    sum=$(sha256sum "$file")
    printf "  '%s'\n" "${sum%% *}"
  done
  echo ')'
  cat <<'BUILD'

prepare() {
  for file in "${source[@]}"; do
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
  cargo test --frozen
  ./target/release/ai-dikte --self-test
}
package() {
  cd "$srcdir/project"
  DESTDIR="$pkgdir" AI_DIKTE_BINARY=rust/target/release/ai-dikte sh packaging/install-linux.sh
}
BUILD
} >> PKGBUILD.tmp
mv PKGBUILD.tmp PKGBUILD
