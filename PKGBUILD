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
  '1027173fcec47fe37d61e2caf508fdcfceaf0128f6379ab4ffc61c3d9c52dcf2'
  '7030648950bc09c7b37a3d62a42b027e356838b36173acd50a80e2205395154d'
  '54a62468d0b8339cbe5ea1b215110da2a772bf85e9c85b3f298dad684516e77e'
  '4d78370f4cf705dbc99515ae2fcfaa56348e41da7a4bc0732cd75571601478fe'
  '56fa7ff02fd1a1a0d5c799e93c2e0ba4ddf2c58fdb525299cdbe1e5a2e1e2417'
  '479ac23ea155575f0fd6acc88454d00e15089778661e8464112e48a5b162d5d9'
  'bae30c17bcc1d5148587356e7788df53c5cae5339ea7ac9a785ef29b17fe5907'
  'ee0647942385ef94d49e6de81c3588af283dc629741f478a21264c088b63c165'
  'c5335a69002bfcd11a9af85221c8e596f0fddff4bf400517b5f015dc3022d4ff'
  '80d4bcb373c2d4af47cf864841d797921d4d910ab63323185ee96be2cb98f04b'
  '0d7c454db85be82363e571ea58604169c89befecfad0e29a37d318840b591623'
  '101e38135ece42a197abd98bfe1535a8fd1fa546169e6ba0988779884549dac4'
  '82daee74b7c59c7226acbfd692f1f3cbad503ffe09caeacf674ef97aa9569412'
  '295b74098ad531efafe1c488900285676964d42ce7afe830520c669134f00d10'
  '2d5562d3e4cc693a238464bc93a415812ee14f90844e51086217aebb3a60dcf8'
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
