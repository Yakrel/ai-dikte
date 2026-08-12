pkgname=gemini-dikte
pkgver=0.2.0
pkgrel=1
pkgdesc='Minimal KDE Plasma Wayland dictation using Gemini'
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
  'qt6-base'
  'wayland'
)
makedepends=('meson' 'ninja' 'pkgconf')
source=(
  'gemini-dikte'
  'gemini-dikte-toggle'
  'gemini-dikte.desktop'
  'LICENSE'
  "kwtype-${_kwtype_commit}.tar.gz::https://github.com/Sporif/KWtype/archive/${_kwtype_commit}.tar.gz"
)
sha256sums=(
  'd255c624d2c37b832b408f93dcc0854f4f15ff6919e00fe60a6449273a47c03f'
  '0199d872c6f3939a7bbd887e997982bdf3352a251bdec65a83071e9330bb35d0'
  '6f5d84dd85631fb3022339c5019bd3271689ad55d787a1da8c641be1bfb4f17d'
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

  install -Dm755 gemini-dikte "$pkgdir/usr/bin/gemini-dikte"
  install -Dm755 gemini-dikte-toggle "$pkgdir/usr/bin/gemini-dikte-toggle"
  install -Dm644 gemini-dikte.desktop \
    "$pkgdir/usr/share/applications/gemini-dikte.desktop"
  install -Dm644 gemini-dikte.desktop \
    "$pkgdir/usr/share/kglobalaccel/gemini-dikte.desktop"
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
