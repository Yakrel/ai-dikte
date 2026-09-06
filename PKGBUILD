pkgname=ai-dikte
pkgver=0.4.0
pkgrel=5
pkgdesc='Minimal Wayland dictation using Gemini 3.5 Transcribe Live'
arch=('any')
url='https://github.com/Yakrel/ai-dikte'
license=('MIT')
depends=(
  'libnotify'
  'pipewire-audio'
  'python'
  'python-websockets'
  'tk'
  'xorg-xwayland'
)
optdepends=(
  'wtype: direct text injection on Hyprland / Omarchy'
  'ai-dikte-kwtype: direct text injection on KDE Plasma Wayland'
)
source=(
  'ai-dikte'
  'ai_dikte.py'
  'ai_dikte_core.py'
  'ai_dikte_config.py'
  'ai_dikte_ui.py'
  'ai-dikte-settings.desktop'
  'ai-dikte.png'
  'ai-dikte-toggle'
  'ai-dikte.desktop'
  'LICENSE'
)
sha256sums=(
  '9b5216a1ffc1ac525fa422140c6a291ecb80f59fd80f6a16722bd6172e615b48'
  '461cdf3e54e975a8b4f7e3abe1a819672f82c682c1189280c9bb7bb7f8a66581'
  'def01b039d228e9ad360d7f4d04b8f4fff74de544f204b0d91f2a45e5369c1a4'
  '4f3a226577f24433afd1144e198c77a7b6c7d04454235c41907d4f37d5e5a4ee'
  'f6239ece1a0c35a64c63aa650f18ce380839b2b72b496e8d08ae9c09c8c569ec'
  'a22f3502c74de8a00cf2193ae90c7e695ec2b5d4e16cde6a24c1ce4746af5ef9'
  '09df0758103426f42ce70aaf495f8740472a09ea73eb84ebfadeae0f2a7017ca'
  'e5fd2f221e661594b9b7c6ab9c1b0c0840b611fe9787cede188911a46f870a55'
  '4af2eea874ada16c8c13e7fb67e7c28c49ec76e8bac7e2997f78f94cc1041134'
  'f8719185a1f3d2a8ec0bf8507b1476e1a0f37cd10328402661f0cd2748d855d3'
)

package() {
  install -Dm755 ai-dikte "$pkgdir/usr/bin/ai-dikte"
  for module in ai_dikte.py ai_dikte_core.py ai_dikte_config.py ai_dikte_ui.py; do
    install -Dm644 "$module" "$pkgdir/usr/lib/ai-dikte/$module"
  done
  install -Dm644 ai-dikte-settings.desktop "$pkgdir/usr/share/applications/ai-dikte-settings.desktop"
  install -Dm644 ai-dikte.png "$pkgdir/usr/lib/ai-dikte/ai-dikte.png"
  install -Dm644 ai-dikte.png "$pkgdir/usr/share/icons/hicolor/256x256/apps/ai-dikte.png"
  install -Dm755 ai-dikte-toggle "$pkgdir/usr/bin/ai-dikte-toggle"
  ln -sf ai-dikte "$pkgdir/usr/bin/gemini-dikte"
  ln -sf ai-dikte-toggle "$pkgdir/usr/bin/gemini-dikte-toggle"

  install -Dm644 ai-dikte.desktop \
    "$pkgdir/usr/share/applications/ai-dikte.desktop"
  install -Dm644 ai-dikte.desktop \
    "$pkgdir/usr/share/kglobalaccel/ai-dikte.desktop"
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
