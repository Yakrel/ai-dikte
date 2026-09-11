pkgname=ai-dikte
pkgver=0.5.0
pkgrel=1
pkgdesc='Rust voice dictation for KDE Plasma and Hyprland using Gemini Live'
arch=('x86_64')
url='https://github.com/Yakrel/ai-dikte'
license=('MIT')
depends=('libnotify' 'pipewire-audio' 'systemd')
makedepends=('rust>=1.95' 'cargo' 'pkgconf')
optdepends=('wtype: Hyprland text injection' 'ai-dikte-kwtype: KDE Plasma text injection')
# Local source files and SHA256 values are generated below; no network source or SKIP hashes.
# BEGIN GENERATED SOURCES
_sources=(
  'rust/Cargo.toml'
  'rust/Cargo.lock'
  'rust/build.rs'
  'rust/src/activity.rs'
  'rust/src/audio.rs'
  'rust/src/background.rs'
  'rust/src/config.rs'
  'rust/src/controller.rs'
  'rust/src/diagnostics.rs'
  'rust/src/lib.rs'
  'rust/src/linux.rs'
  'rust/src/live.rs'
  'rust/src/main.rs'
  'rust/src/output.rs'
  'rust/src/protocol.rs'
  'rust/src/session.rs'
  'rust/src/settings.rs'
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
  '1c7a2fa6914bee5eaf118ecc2ce9435af280ac36e5bba41605122419e431294d'
  '6dcfd15fb0b83383bcdbf419e9a995979dd5d4cfeb4e5322c23765807c4325a7'
  '5a9ce8027186e1ea80c881ec71c7b7433bcec1f037cd81be42a882ece5923588'
  '64123bb9a7c61286f3876d9185adc4c2f032f668f62e2ef216f60f8b3ee3e3ae'
  '1027173fcec47fe37d61e2caf508fdcfceaf0128f6379ab4ffc61c3d9c52dcf2'
  '7030648950bc09c7b37a3d62a42b027e356838b36173acd50a80e2205395154d'
  '54a62468d0b8339cbe5ea1b215110da2a772bf85e9c85b3f298dad684516e77e'
  'ff2678620e95243c1a7bcfcf43c16821ecb2c7b556bb03bef46c102819c6cf40'
  '56fa7ff02fd1a1a0d5c799e93c2e0ba4ddf2c58fdb525299cdbe1e5a2e1e2417'
  'eef69078824f7f8a95c39b922b7f65fa7480d308783ec70ddc592f966c01a9e6'
  'bae30c17bcc1d5148587356e7788df53c5cae5339ea7ac9a785ef29b17fe5907'
  '986c539299f245ea311cf2b58ccd83fa8fe9979d296fd9b5100f0c3c5bca34a2'
  '654e13dc9b59caa737345a704b732d1e9fd0ef494957e222cbac84e5571471fe'
  '80d4bcb373c2d4af47cf864841d797921d4d910ab63323185ee96be2cb98f04b'
  '5d96b067bcd64b91fcf303b6a6863116d706587ee045331dfa233ebc4a2f34b8'
  '101e38135ece42a197abd98bfe1535a8fd1fa546169e6ba0988779884549dac4'
  '82daee74b7c59c7226acbfd692f1f3cbad503ffe09caeacf674ef97aa9569412'
  '295b74098ad531efafe1c488900285676964d42ce7afe830520c669134f00d10'
  'TO_BE_REGENERATED'
  'bf57dd9db73b223ea53641ca278ba9b48561939d5b46599b6746898eac24d825'
  '09df0758103426f42ce70aaf495f8740472a09ea73eb84ebfadeae0f2a7017ca'
  '2ae98422757c1ec7225d311ee913eaf029f0f091399c58f8a76ccda2d1d51254'
  'e5fd2f221e661594b9b7c6ab9c1b0c0840b611fe9787cede188911a46f870a55'
  '4af2eea874ada16c8c13e7fb67e7c28c49ec76e8bac7e2997f78f94cc1041134'
  'aa546fd695a22fd2fa3e0b4edb6c7dee2963747d05075ae57afcef7af9ab8331'
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
  export CFLAGS+=" -ffat-lto-objects"
  export CXXFLAGS+=" -ffat-lto-objects"
  cd "$srcdir/project/rust"
  cargo build --frozen --release
}
check() {
  export CFLAGS+=" -ffat-lto-objects"
  export CXXFLAGS+=" -ffat-lto-objects"
  cd "$srcdir/project/rust"
  cargo test --frozen --release
  ./target/release/ai-dikte --self-test
}
package() {
  cd "$srcdir/project"
  DESTDIR="$pkgdir" AI_DIKTE_BINARY=rust/target/release/ai-dikte sh packaging/install-linux.sh
}
