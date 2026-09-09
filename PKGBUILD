pkgname=ai-dikte
pkgver=0.5.0
pkgrel=1
pkgdesc='Rust voice dictation for KDE Plasma and Hyprland using Gemini Live'
arch=('x86_64')
url='https://github.com/Yakrel/ai-dikte'
license=('MIT')
depends=('libnotify' 'pipewire-audio' 'libxkbcommon' 'libxkbcommon-x11' 'wayland' 'libglvnd' 'libx11' 'libxcursor' 'libxi' 'libxrandr')
makedepends=('rust>=1.95' 'cargo' 'pkgconf')
optdepends=('wtype: Hyprland text injection' 'ai-dikte-kwtype: KDE Plasma text injection')
# Local source files and SHA256 values are generated below; no network source or SKIP hashes.
# BEGIN GENERATED SOURCES
_sources=(
  'rust/Cargo.toml'
  'rust/Cargo.lock'
  'rust/build.rs'
  'rust/src/audio.rs'
  'rust/src/config.rs'
  'rust/src/controller.rs'
  'rust/src/cue.rs'
  'rust/src/diagnostics.rs'
  'rust/src/lib.rs'
  'rust/src/linux.rs'
  'rust/src/live.rs'
  'rust/src/main.rs'
  'rust/src/output.rs'
  'rust/src/protocol.rs'
  'rust/src/session.rs'
  'rust/src/shortcut.rs'
  'rust/src/ui.rs'
  'rust/src/windows.rs'
  'ai-dikte.png'
  'ai-dikte.ico'
  'ai-dikte-toggle'
  'ai-dikte.desktop'
  'ai-dikte-settings.desktop'
  'packaging/install-linux.sh'
  'packaging/linux/ai-dikte.service'
  'LICENSE'
)
# makepkg resolves local sources by basename. Explicit file URLs retain the
# checkout path for files in subdirectories while checksums remain mandatory.
DLAGENTS+=('file::/usr/bin/curl -qg -o %o %u')
source=()
for file in "${_sources[@]}"; do
  source+=("${file##*/}::file://$startdir/$file")
done
sha256sums=(
  'e13976c268392329b9840529c8f222bf9e01e0569a190a08fca4958113964e06'
  '621b65b767ac8bf98bc7e6b68eab47bef104df3141a1e1b1094e7557e49381a7'
  '5a9ce8027186e1ea80c881ec71c7b7433bcec1f037cd81be42a882ece5923588'
  '1027173fcec47fe37d61e2caf508fdcfceaf0128f6379ab4ffc61c3d9c52dcf2'
  '54a62468d0b8339cbe5ea1b215110da2a772bf85e9c85b3f298dad684516e77e'
  '6d352f15cd1ffc374002b06cdab1b9372bf1f31d7264c7d19ce9623907dca1ae'
  'bbef59ce4e53586681f972e961b627f1ad6310b325f7c82c4c0742defd7f4ebe'
  '401a8d11833fade9cd8e859a21f0c53d7f36eefc05e951bdc8d30a4ff367bf67'
  'f500bec14f5466da20c57cad3409aad63d890496b01c4d2743f54ee30e39a9de'
  'a7a2008aaee844d36a63aa82f6e346fea3115d7bbb47887dc95e97a3a1ff1711'
  'ee0647942385ef94d49e6de81c3588af283dc629741f478a21264c088b63c165'
  '7d5bab6005c22a1d82bfdd561eb6ad883eb961fbd95898f187cbe8b00916232e'
  '80d4bcb373c2d4af47cf864841d797921d4d910ab63323185ee96be2cb98f04b'
  '0d7c454db85be82363e571ea58604169c89befecfad0e29a37d318840b591623'
  'e8e7202a1854bac0836e3e07e1191feed7d751fb49a6a0f3bef8d9f9e92b331f'
  '295b74098ad531efafe1c488900285676964d42ce7afe830520c669134f00d10'
  '3341e6c0ab2526550d3e34bee8d8d4085dd0feb86cf6e76862f5cc84383c90e7'
  'a507144476be4c57e63ba13734641a51e4b264be725db404f71c4e8df69c34ad'
  '09df0758103426f42ce70aaf495f8740472a09ea73eb84ebfadeae0f2a7017ca'
  '2ae98422757c1ec7225d311ee913eaf029f0f091399c58f8a76ccda2d1d51254'
  'e5fd2f221e661594b9b7c6ab9c1b0c0840b611fe9787cede188911a46f870a55'
  '4af2eea874ada16c8c13e7fb67e7c28c49ec76e8bac7e2997f78f94cc1041134'
  'a22f3502c74de8a00cf2193ae90c7e695ec2b5d4e16cde6a24c1ce4746af5ef9'
  'aa9d4b94d3822197e53bf34ae3a4c435cb20939f7a538ad313365b963edbbf2a'
  'aa9ed30a060dfa449e27baa7b03924753146568ceb22f156d87b3f890ab05842'
  'f8719185a1f3d2a8ec0bf8507b1476e1a0f37cd10328402661f0cd2748d855d3'
)

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
