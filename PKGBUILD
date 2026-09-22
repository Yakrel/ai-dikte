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
  'rust/tests/cli.rs'
  'rust/tests/fixtures/short-speech.pcm'
  'rust/tests/fixtures/speech.pcm'
  'rust/tests/live_api.rs'
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
  '5c504be37c7a1607dea03e51fff31e45621cc7f94feca60cc2009e3725d25d15'
  '45aeddf57ff4bd32c5aa1a6a98013694e7b489616e5ce4e6c748ca7ab78443b9'
  '5a9ce8027186e1ea80c881ec71c7b7433bcec1f037cd81be42a882ece5923588'
  '3283f1643748449b06aa6f8adc39e259842bd5f000f8b5bf8a50efa9bb6f7963'
  'd13ee9e126702a7b5cbe91667ba32f9cfd739c8eca641a8062c2d1e5ae7022c9'
  'eef3cc76368131ed50554b332130f4822180d3e7e65692ff2acb23a6660af1fe'
  '700034e4766f499daa0af2ed51df7c0e0502bd0fee6a411e63108403d0d07648'
  'b3766fbe9b3eaaa7008f9f001fc912912c319f4b835934ee2f060e36ac3e44cf'
  '56fa7ff02fd1a1a0d5c799e93c2e0ba4ddf2c58fdb525299cdbe1e5a2e1e2417'
  'eef69078824f7f8a95c39b922b7f65fa7480d308783ec70ddc592f966c01a9e6'
  'fcc9d07025f67b08e21f9efa819d0bf23d68c1d14e864c2ec148d15a7c565d80'
  '1e2c3c372d57acae2b65f28618885b192930d94c0d3e58acafb97991f0ec19de'
  'ef6632dd8c94ece53d481fd498cfb8b2699afcf2f70cc1e4bf277ccd9e1de960'
  '8334f84d538131fb085c5ee89d58ccc69ba693b620e25279533a9f417fa9ec8a'
  'b7a7ffed0d23adb088cbce4b9a65a731b619c77ba3cd502571ea5542761a968d'
  '1d43708a273f4e5ed1a39b9f2038c494c6236a3bbd92e071e8f3beccd0789cbc'
  'a8bd7b820f5e7cace21a266b6e90afaf6ce2a547f06028276fd01950ca1c1009'
  'd1ad1277a886afc2aeb7db3703d8ba1fc3ceec1f2f34af6c45b43426819318e5'
  '8b7e50b40425db6108702f9b914d4e5ae3215a447341297fcbb15bb414388259'
  '6a0f47dd31d7abc8b5c9787ce50957a2fa06275597a09bb1bbe73b1b3aa51b05'
  '765eefa304a3f9528aa080257fb1d99ed731aa1901cbbbbc6a2937ea992908a5'
  'dc8b2d70cb6e215fb0e9fdc6c3f4fea7dce45263e58e032c468065fae3ecc2db'
  'f3d7f853a7c04c2e736b27939a39ef896f82d802f483cfe4569a08e3f78bea65'
  '0eb1f570a4d3319d37acdf85d52cf2ce53577456db61b77b6c54fd41aa60fb80'
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
