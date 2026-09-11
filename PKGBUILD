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
  'a6aa2a902fe8a385207c3a30658127a6c80fb7f138594a0aee08e3d303b8760f'
  'eef3cc76368131ed50554b332130f4822180d3e7e65692ff2acb23a6660af1fe'
  '54a62468d0b8339cbe5ea1b215110da2a772bf85e9c85b3f298dad684516e77e'
  'ff2678620e95243c1a7bcfcf43c16821ecb2c7b556bb03bef46c102819c6cf40'
  '56fa7ff02fd1a1a0d5c799e93c2e0ba4ddf2c58fdb525299cdbe1e5a2e1e2417'
  'eef69078824f7f8a95c39b922b7f65fa7480d308783ec70ddc592f966c01a9e6'
  '92b268275eb32ffbe87aa007a0d04c8866eccb7d068e6a51da8fcf7245d0bb9c'
  '25dae628d6b4bf80bfa8f96734e7194de1082d5256f91b2514b39ba89da14682'
  'ef6632dd8c94ece53d481fd498cfb8b2699afcf2f70cc1e4bf277ccd9e1de960'
  '8334f84d538131fb085c5ee89d58ccc69ba693b620e25279533a9f417fa9ec8a'
  '5d96b067bcd64b91fcf303b6a6863116d706587ee045331dfa233ebc4a2f34b8'
  '595d08a254ad1375d94e47710abd70497932bd37df4cd195251b6df91683c7ee'
  '82daee74b7c59c7226acbfd692f1f3cbad503ffe09caeacf674ef97aa9569412'
  '295b74098ad531efafe1c488900285676964d42ce7afe830520c669134f00d10'
  '181ce89de479538881832371a8168fef37b7fc61c3736e8a807dd9ee4bfb492b'
  '05fc243e12c530a585d81772c6210222b98645478e71a424bb5238f9849944a8'
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
