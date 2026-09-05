pkgname=ai-dikte
pkgver=0.4.0
pkgrel=3
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
  '794befdd6d9112c1824c5ea16d7a58172343a14483345cdcef98097c564c44ef'
  '3799cf6086600cbc25237ebc22fb5c53618a560b0383ddce8e2fe56d345da655'
  '53e750d66e69db037aef3dd0657028b12bbfcd878f08425c90cfa4e31b331730'
  '9435c68cf6d40a4b774a26921cc1da82bc7c7e074b1a05f9456334a75a3b5413'
  '4cc51fb14cb3b73bec99067f077fd225da3d1413b871012442151ead0e05bd68'
  'e27968a1bebdc3ee6904a17718b13169580e8f3c8eb94a3a23888122d684ffdb'
  '3f307d6506708e7e64884289ae88d7543f690dc9f4fdda2a5b239c1783cd5233'
  'bc9078179cb885b376f3fd137e20e17359cab66dc1670ffd1393a84aea453f9e'
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
