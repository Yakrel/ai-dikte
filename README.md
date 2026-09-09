# AI Dikte

Windows ve Linux Wayland için Rust ile yazılmış Gemini sesli yazma uygulaması.
**Win+Z / Meta+Z** ile kaydı başlatın; tekrar basınca kesinleşmiş metin odaktaki uygulamaya yazılır.

> Rust geçişi PR #8 üzerinde geliştiriliyor. Gerçek cihaz kabul testleri bitene kadar draft durumundadır. Yapılanlar ve kalanlar: [RUST_MIGRATION.md](RUST_MIGRATION.md).

## Özellikler

- Gemini `gemini-3.5-transcribe-live`, Türkçe varsayılan dil, SMART / VERBATIM ve özel kelimeler.
- Windows: WASAPI mikrofon, Win+Z hook, doğrudan Unicode SendInput, tray, odak çalmayan durum göstergesi, başlangıç seçeneği.
- Linux: PipeWire kayıt; KDE Plasma'da yalnız **kwtype**, Hyprland / Omarchy'de yalnız **wtype**.
- Ortak egui ayar ekranı; Linux'ta 0600 izinli atomik JSON kaydı, Windows'ta Credential Manager API anahtarı.
- Kayıt/bağlantı/çıktı hatalarında açık hata; otomatik alternatif backend veya Python kurulumu yok.
- Sesli bildirim varsayılan kapalı. Bildirim ve ses tercihleri bağımsızdır.

## Windows

Rust sürümü main'e alındıktan sonra PowerShell'de:

```powershell
irm https://raw.githubusercontent.com/Yakrel/ai-dikte/main/install.ps1 | iex
```

Installer EXE ve SHA256 dosyasını doğrular, runtime self-test çalıştırır, kullanıcı hesabına kurar ve ayar ekranını açar. Yönetici yetkisi veya Python gerekmez. Otomatik başlangıç tray menüsünden açılır.

**PR sürümünü main'e almadan test etmek için:** PR'ın **Build Windows Executable** Actions koşusundaki `ai-dikte-windows` artifact'ini indirin ve ZIP'i açın. Eski daemon'dan çıkın. Doğrudan `ai-dikte-windows.exe` çalıştırabilirsiniz. İlk açılışta ayarlar açılır; kaydedip kapatınca daemon başlar. İsterseniz bu daldaki installer ile:

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

## Komutlar

| Komut | İşlev |
|---|---|
| `ai-dikte setup` | Ayarlar |
| `ai-dikte daemon` | Tek daemon; Windows tray / Linux kullanıcı servisi |
| `ai-dikte toggle` | Linux daemon'a başlat/durdur isteği |
| `ai-dikte doctor` | Anahtarı göstermeyen tanılama raporu |
| `ai-dikte check-config` | Kurulum için yapılandırma/aygıt/backend kontrolü; hatada nonzero |
| `ai-dikte logs` | Oturum günlüğü |
| `ai-dikte --self-test` | Anahtar, mikrofon ve ağ gerektirmeyen runtime kontrolü |
| `ai-dikte shortcut-install` / `shortcut-remove` | Hyprland'daki yönetilen kısayol bloğu |

Ayarlar: Windows `%APPDATA%\ai-dikte\config.json`; Linux `$XDG_CONFIG_HOME/ai-dikte/config.json` (varsayılan `~/.config/ai-dikte/config.json`). `session.log` aynı dizindedir; ses ve transkript kaydedilmez. Linux kontrol soketi `$XDG_RUNTIME_DIR/ai-dikte-rust/control.sock` içindedir.

`doctor` bağlantı veya mikrofon kaydı testi yapmaz. `check-config` API anahtarının bulunmasını ve yerel gereksinimleri kontrol eder; Gemini doğrulaması API ayarlarını kaydederken yapılır. Windows yükseltilmiş uygulamalara yazmayı engelleyebilir; SendInput hatası açıkça gösterilir.

## Geliştirme

Rust 1.98+ ve platform derleme araçları gerekir. Windows ikon kaynağı için Windows SDK gerekir. Linux GUI kütüphaneleri: Wayland, XKB, X11 ve OpenGL/EGL.

```sh
cd rust
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked --release
```

Rust kodu veya paketlenen dosyalar değişince kökte `packaging/update-arch-sources.sh` çalıştırarak PKGBUILD SHA256 değerlerini yenileyin. Python uygulaması kaldırıldı; önceki sürüm Git geçmişinde bulunur. Paket/CI ve gerçek cihaz doğrulamasının durumu [kontrol listesinde](RUST_MIGRATION.md) tutulur.
