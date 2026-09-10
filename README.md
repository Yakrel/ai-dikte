# AI Dikte

Windows ve Linux Wayland için Rust ile yazılmış Gemini sesli yazma uygulaması.
**Win+Z / Meta+Z** ile kaydı başlatın; tekrar basınca kesinleşmiş metin odaktaki uygulamaya yazılır.

> Rust geçişi PR #8 üzerinde geliştiriliyor. Gerçek cihaz kabul testleri bitene kadar draft durumundadır. Yapılanlar ve kalanlar: [RUST_MIGRATION.md](RUST_MIGRATION.md).

## Özellikler

- Gemini `gemini-3.5-transcribe-live`, Türkçe varsayılan dil, SMART / VERBATIM ve özel kelimeler.
- Windows: WASAPI mikrofon, Win+Z hook, doğrudan Unicode SendInput, tray, odak çalmayan durum göstergesi, başlangıç seçeneği.
- Linux: PipeWire kayıt; KDE Plasma'da yalnız **kwtype**, Hyprland / Omarchy'de yalnız **wtype**.
- Windows ve Linux için ortak Ratatui terminal menüsü; Linux'ta 0600 izinli atomik JSON kaydı, Windows'ta Credential Manager API anahtarı.
- Kayıt/bağlantı/çıktı hatalarında açık hata; otomatik alternatif backend veya Python kurulumu yok.
- Sesli bildirim varsayılan kapalı. Bildirim ve ses tercihleri bağımsızdır.

Arayüz İngilizcedir; **Spoken language** yalnız konuşma dilini belirler. **Smart** dolgu sözcüklerini temizleyip metni düzenler; **Verbatim** tekrarlar ve dolgu sözcükleri dahil söylenenleri korur. [Gemini mod açıklamaları](https://ai.google.dev/gemini-api/docs/live-api/live-transcribe).

## Windows

Rust sürümü main'e alındıktan sonra PowerShell'de:

```powershell
irm https://raw.githubusercontent.com/Yakrel/ai-dikte/main/install.ps1 | iex
```

Installer terminal uygulaması ve konsolsuz arka plan çalıştırıcısının SHA256 değerlerini doğrular, her ikisinin self-test'ini çalıştırır, kullanıcı hesabına kurar ve TUI ayarlarını açar. Yönetici yetkisi veya Python gerekmez. Otomatik başlangıç tray menüsünden açılır.

**PR sürümünü main'e almadan test etmek için:** PR'ın **Build Windows Executable** Actions koşusundaki `ai-dikte-windows` artifact'ini indirin ve ZIP'i açın. Eski daemon'dan çıkın. ZIP içindeki iki EXE ve iki SHA256 dosyasını aynı klasörde tutun. Bu daldaki installer dosyaları doğru adlarla kurar:

```powershell
.\install.ps1 -ArtifactDirectory C:\Path\To\ExtractedArtifact
```

Mikrofonu ayarlar listesinden seçin. Seçilen aygıt kaybolursa başka mikrofon kullanılmaz; sistem varsayılanını açıkça seçebilirsiniz. Aynı isimli birden fazla aygıt varsa Windows varsayılan giriş aygıtını kullanın. Eski sayısal `input_device` ayarı varsa `config.json` içinden bu alanı kaldırıp yeniden seçin; eski ayarları taşıma kodu yoktur.

## Arch / CachyOS / Omarchy

Rust dalı main'e alındıktan sonra:

```sh
bash -c "$(curl -fsSL https://raw.githubusercontent.com/Yakrel/ai-dikte/main/install.sh)"
```

Installer mevcut Wayland oturumuna göre gereken typing backend'i kurar, Rust paketini derler, ayarları açar ve kullanıcı daemon servisini etkinleştirir. Hyprland kısayol bloğu eklenir; yapılandırma hatasında değişiklik geri alınır.

Bu dalı kaynaktan kurmak için:

```sh
makepkg --syncdeps --install
ai-dikte setup
ai-dikte check-config
systemctl --user import-environment WAYLAND_DISPLAY XDG_CURRENT_DESKTOP XDG_SESSION_DESKTOP DESKTOP_SESSION
# Hyprland kullanıyorsanız:
# systemctl --user import-environment HYPRLAND_INSTANCE_SIGNATURE
# ai-dikte shortcut-install
systemctl --user daemon-reload
systemctl --user enable --now ai-dikte.service
```

KDE için ayrıca `PKGBUILD.kwtype` ile KWtype kurun; Hyprland için `wtype` kurun. Çıplak Hyprland oturumlarında kullanıcı systemd yöneticisine Wayland ortamı her oturumda aktarılmalıdır; KDE/uwsm bunu oturum yönetimiyle sağlar.

## Fedora KDE

**Fedora KDE packages** Actions artifact'indeki iki RPM'i (`ai-dikte` ve `kwtype`) birlikte kurun:

```sh
sudo dnf install ./*.rpm
ai-dikte setup
ai-dikte check-config
systemctl --user import-environment WAYLAND_DISPLAY XDG_CURRENT_DESKTOP XDG_SESSION_DESKTOP DESKTOP_SESSION
systemctl --user daemon-reload
systemctl --user enable --now ai-dikte.service
```

RPM kaynaktan Rust derler. CI, Cargo.lock ile bağımlılıkları önceden vendor eder; rpmbuild ağ erişimi olmadan `--frozen` çalışır.

## NixOS

Flake paketleri: `ai-dikte-kde`, `ai-dikte-hyprland`, `kwtype`.

```sh
nix build .#ai-dikte-kde
./result/bin/ai-dikte --self-test
```

NixOS yapılandırmanızda seçilen paketi `environment.systemPackages` ve `systemd.packages` içine ekleyin. Kullanıcınızla `ai-dikte setup` çalıştırdıktan sonra `systemctl --user enable --now ai-dikte.service` kullanın. Wayland oturum ortamı kullanıcı systemd yöneticisine aktarılmış olmalıdır. Paket yalnız kendi masaüstü backend'ini PATH'e ekler; çalışma anında Python kullanmaz.

## Terminal menüsü

`ai-dikte` veya `ai-dikte menu` ile açılır. TUI İngilizcedir ve kayıt başlat/durdur içermez; kayıt **Win+Z / Meta+Z** ile yapılır. Menü kapanınca çalışan daemon etkilenmez. Windows'ta argümansız açılışta, ayarlar kaydedilmişse menüden çıktıktan sonra arka plan uygulaması çalıştırılır. Linux'ta daemon kurulumun etkinleştirdiği kullanıcı systemd servisi tarafından yönetilir.

| Bölüm | İşlev |
|---|---|
| Status | Arka plan uygulaması, mikrofon ve model bilgisi |
| Settings | API anahtarı, dil, Smart/Verbatim, mikrofon ve özel kelimeler |
| Preferences | Ses, bildirim ve oturum açılışında başlatma |
| Diagnostics | T: kayıtlı API bağlantısını test et; C: yerel cihaz/backend kontrolü |
| Logs | Oturum günlüğünü kaydırarak oku |
| Exit menu | Yalnız menüyü kapat |

- **↑/↓**, **Enter**: gezin ve seç. Mikrofon alanında Enter sonraki aygıtı seçer; listenin sonunda sistem varsayılanına döner. **R** listeyi yeniler.
- Metin alanında **←/→**, **Home/End**, **Backspace/Delete**; **Ctrl+U** temizler. API anahtarı sürekli maskelenir.
- **Enter** düzenlemeyi taslağa uygular; **Esc** o düzenlemeyi iptal eder. Özel kelimeler için satır başına bir terim yapıştırın veya **Alt+Enter** kullanın.
- **Ctrl+S** bağlantıyı doğrular ve kaydeder. Hatalı kayıtta önceki ayarlar korunur; taslak kaybolmaz.
- **Esc** ana menüye döner, **Q** çıkar. Kaydedilmemiş taslak varsa **Y** ile silerek çıkış onaylanır.
- Oturum açılışında başlatma tercihi hemen uygulanır; ses/bildirim tercihleri Ctrl+S ile kaydedilir. Bu seçenek mevcut kaydı veya daemon'ı başlatıp durdurmaz.
- TUI etkileşimli terminal ister. Pipe/script kullanımı için aşağıdaki düz metin komutları vardır.

Windows paketi `ai-dikte.exe` (konsol/TUI) ve `ai-dikte-background.exe` (tray/Win+Z, konsolsuz) içerir. İkisi aynı Rust kütüphanesini kullanır ve birlikte kurulmalıdır. Tray'deki ayar/tanılama/günlük seçenekleri yeni bir terminal açar.

## Komutlar

| Komut | İşlev |
|---|---|
| `ai-dikte` / `ai-dikte menu` | TUI ana menüsü |
| `ai-dikte setup` | TUI ayarları |
| `ai-dikte status` | Düz metin daemon ve ayar durumu |
| `ai-dikte diagnostics` / `ai-dikte log-view` | TUI tanılama / günlük ekranı |
| `ai-dikte daemon` | Tek daemon; Windows tray / Linux kullanıcı servisi |
| `ai-dikte toggle` | Linux daemon'a başlat/durdur isteği |
| `ai-dikte doctor` | Anahtarı göstermeyen tanılama raporu |
| `ai-dikte check-config` | Kurulum için yapılandırma/aygıt/backend kontrolü; hatada nonzero |
| `ai-dikte logs` | Oturum günlüğü |
| `ai-dikte --self-test` | Anahtar, mikrofon ve ağ gerektirmeyen runtime kontrolü |
| `ai-dikte shortcut-install` / `shortcut-remove` | Hyprland'daki yönetilen kısayol bloğu |

Ayarlar: Windows `%APPDATA%\ai-dikte\config.json`; Linux `$XDG_CONFIG_HOME/ai-dikte/config.json` (varsayılan `~/.config/ai-dikte/config.json`). `session.log` aynı dizindedir; ses ve transkript kaydedilmez. Linux kontrol soketi `$XDG_RUNTIME_DIR/ai-dikte-rust/control.sock` içindedir.

`doctor` bağlantı veya mikrofon kaydı testi yapmaz. `check-config` API anahtarının bulunmasını ve yerel gereksinimleri kontrol eder; Gemini doğrulaması her **Ctrl+S** işleminde yapılır; bağlantı veya doğrulama başarısızsa ayarlar kaydedilmez. Windows yükseltilmiş uygulamalara yazmayı engelleyebilir; SendInput hatası açıkça gösterilir.

## Geliştirme

Rust 1.95+ ve platform derleme araçları gerekir. Windows ikon kaynağı için Windows SDK gerekir. TUI için grafik çizim kütüphanesi gerekmez; gerçek dikte için Linux'ta PipeWire ve masaüstüne uygun typing backend gerekir.

```sh
cd rust
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked --release
# Windows paketinin iki çalıştırıcısını derlemek için:
# cargo build --locked --release --features windows-host
```

Rust kodu veya paketlenen dosyalar değişince kökte `packaging/update-arch-sources.sh` çalıştırarak PKGBUILD SHA256 değerlerini yenileyin. Python uygulaması kaldırıldı; önceki sürüm Git geçmişinde bulunur. Paket/CI ve gerçek cihaz doğrulamasının durumu [kontrol listesinde](RUST_MIGRATION.md) tutulur.
