pkgname=ai-dikte
pkgver=0.3.0
pkgrel=1
pkgdesc='Minimal Wayland dictation using Gemini 3.5 Transcribe Live'
arch=('x86_64')
url='https://github.com/Yakrel/ai-dikte'
license=('MIT')
_kwtype_commit='ac2c3864aaacc31afc252d88d1d4b669270f2f44'
depends=(
  'kwayland'
  'libnotify'
  'libxkbcommon'
  'pipewire-audio'
  'python'
  'python-websockets'
  'qt6-base'
  'wayland'
  'wtype'
)
makedepends=('meson' 'ninja' 'pkgconf')
source=(
  'ai-dikte'
  'ai-dikte-toggle'
  'ai-dikte.desktop'
  'LICENSE'
  "kwtype-${_kwtype_commit}.tar.gz::https://github.com/Sporif/KWtype/archive/${_kwtype_commit}.tar.gz"
)
sha256sums=(
  'c28c3bb8dcac1daea4193d795bdfe5ff8201bb4847722ed9f1546c16665453bb'
  '3f307d6506708e7e64884289ae88d7543f690dc9f4fdda2a5b239c1783cd5233'
  'bc9078179cb885b376f3fd137e20e17359cab66dc1670ffd1393a84aea453f9e'
  'f8719185a1f3d2a8ec0bf8507b1476e1a0f37cd10328402661f0cd2748d855d3'
  'ec6f1fa5128835dabbbb3f819ba5aea318eb6e81a8ec47d9fb2868af152c7b41'
)

build() {
  local kwtype_dir="KWtype-${_kwtype_commit}"
  arch-meson "$kwtype_dir" build
  meson compile -C build
}

package() {
  DESTDIR="$pkgdir" meson install -C build

  install -Dm755 ai-dikte "$pkgdir/usr/bin/ai-dikte"
  install -Dm755 ai-dikte-toggle "$pkgdir/usr/bin/ai-dikte-toggle"
  ln -sf ai-dikte "$pkgdir/usr/bin/gemini-dikte"
  ln -sf ai-dikte-toggle "$pkgdir/usr/bin/gemini-dikte-toggle"

  install -Dm644 ai-dikte.desktop \
    "$pkgdir/usr/share/applications/ai-dikte.desktop"
  install -Dm644 ai-dikte.desktop \
    "$pkgdir/usr/share/kglobalaccel/ai-dikte.desktop"
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
