pkgname=ai-dikte
pkgver=0.3.0
pkgrel=2
pkgdesc='Minimal Wayland dictation using Gemini 3.5 Transcribe Live'
arch=('x86_64')
url='https://github.com/Yakrel/ai-dikte'
license=('MIT')
depends=(
  'libnotify'
  'pipewire-audio'
  'python'
  'python-websockets'
)
optdepends=(
  'wtype: direct text injection on Hyprland / Omarchy'
  'ai-dikte-kwtype: direct text injection on KDE Plasma Wayland'
)
source=(
  'ai-dikte'
  'ai-dikte-toggle'
  'ai-dikte.desktop'
  'LICENSE'
)
sha256sums=(
  'c28c3bb8dcac1daea4193d795bdfe5ff8201bb4847722ed9f1546c16665453bb'
  '3f307d6506708e7e64884289ae88d7543f690dc9f4fdda2a5b239c1783cd5233'
  'bc9078179cb885b376f3fd137e20e17359cab66dc1670ffd1393a84aea453f9e'
  'f8719185a1f3d2a8ec0bf8507b1476e1a0f37cd10328402661f0cd2748d855d3'
)

package() {
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
