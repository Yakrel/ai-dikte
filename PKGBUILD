pkgname=ai-dikte
pkgver=0.4.0
pkgrel=5
pkgdesc='Minimal Wayland dictation using Gemini 3.5 Transcribe Live'
arch=('x86_64')
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
  'ai-dikte-toggle'
  'ai-dikte.desktop'
  'LICENSE'
)
sha256sums=(
  '9b5216a1ffc1ac525fa422140c6a291ecb80f59fd80f6a16722bd6172e615b48'
  '3799cf6086600cbc25237ebc22fb5c53618a560b0383ddce8e2fe56d345da655'
  '28d735d7ad59e2b6e021f5712258d17da443556b257158005afc990776a48b75'
  'ff49a8edba6850e5d63f67d5dcca1675843bb9b2f85611ea0d6cc8c0deb54f71'
  '4cc51fb14cb3b73bec99067f077fd225da3d1413b871012442151ead0e05bd68'
  '80f7b318a064fdca2ccf3e857bd1d6dd13ac44e9b3947efee3e8b6e80c93c2ba'
  'e5fd2f221e661594b9b7c6ab9c1b0c0840b611fe9787cede188911a46f870a55'
  '62cabc68f86da81333fa68f42c89e01dc12a89a5fc1f416b8c18ac0ea9f6f480'
  'f8719185a1f3d2a8ec0bf8507b1476e1a0f37cd10328402661f0cd2748d855d3'
)

package() {
  install -Dm755 ai-dikte "$pkgdir/usr/bin/ai-dikte"
  for module in ai_dikte.py ai_dikte_core.py ai_dikte_config.py ai_dikte_ui.py; do
    install -Dm644 "$module" "$pkgdir/usr/lib/ai-dikte/$module"
  done
  install -Dm644 ai-dikte-settings.desktop "$pkgdir/usr/share/applications/ai-dikte-settings.desktop"
  install -Dm755 ai-dikte-toggle "$pkgdir/usr/bin/ai-dikte-toggle"
  ln -sf ai-dikte "$pkgdir/usr/bin/gemini-dikte"
  ln -sf ai-dikte-toggle "$pkgdir/usr/bin/gemini-dikte-toggle"

  install -Dm644 ai-dikte.desktop \
    "$pkgdir/usr/share/applications/ai-dikte.desktop"
  install -Dm644 ai-dikte.desktop \
    "$pkgdir/usr/share/kglobalaccel/ai-dikte.desktop"
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
