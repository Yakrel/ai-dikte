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
  'rust/src/background_main.rs'
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
  'f762adf6340f49b427e21c267c046980b1a4318858be9e89aa383adaaf43c0ed'
  '66bf010eb2d540ba092814daf49977a751c4bb4caca2881dc6d0ce12644734ad'
  '5a9ce8027186e1ea80c881ec71c7b7433bcec1f037cd81be42a882ece5923588'
  '1027173fcec47fe37d61e2caf508fdcfceaf0128f6379ab4ffc61c3d9c52dcf2'
  'dafc68f48ab9ebd3d0bd4d35c414126c80a0d150d3f52e52df039345599418b0'
  '20a0a4d083e1294c34b2f44dddc702036a83d9c7766ce551b1fb6fdabe3bc816'
  '54a62468d0b8339cbe5ea1b215110da2a772bf85e9c85b3f298dad684516e77e'
  '6d352f15cd1ffc374002b06cdab1b9372bf1f31d7264c7d19ce9623907dca1ae'
  'bbef59ce4e53586681f972e961b627f1ad6310b325f7c82c4c0742defd7f4ebe'
  '80d4cf6184caf3904dd4917d3c1d7ff0393d63673ee74b8fe8e974da3af89140'
  'df95723000299b1324b2877044f4fee34eae37a0e809c7606450ced5f611085b'
  '6fba95021e835d162073dd164957618e0fd59b314602f8780bba70e36bee623e'
  'ee0647942385ef94d49e6de81c3588af283dc629741f478a21264c088b63c165'
  'de343eccc001e81c9dbd2e3700106c94b4c40f1a19ca10bea6fae82fe5a97a7b'
  '80d4bcb373c2d4af47cf864841d797921d4d910ab63323185ee96be2cb98f04b'
  '0d7c454db85be82363e571ea58604169c89befecfad0e29a37d318840b591623'
  'e8e7202a1854bac0836e3e07e1191feed7d751fb49a6a0f3bef8d9f9e92b331f'
  '82daee74b7c59c7226acbfd692f1f3cbad503ffe09caeacf674ef97aa9569412'
  '295b74098ad531efafe1c488900285676964d42ce7afe830520c669134f00d10'
  '02c67ebb9cc8376928d889d02e76027a57ccb197d50e4907f2f8cbc6f2ec46cb'
  'f6c62e31936be2ffe65cc910b9fca6e5a18b4f9caae512496fe8b95b5da06ef2'
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
