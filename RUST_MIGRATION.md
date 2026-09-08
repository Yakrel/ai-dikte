## Amaç
Python uygulamasını Windows ve Linux için Rust ile yeniden yazmak. Bu PR tamamlanana kadar draft kalır.

## Kontrol listesi
- [x] main @ 24c872e mevcut davranış ve platform envanteri
- [x] Rust proje yapısı, kilitli bağımlılıklar, CI tanımı
- [x] Ayar doğrulama ve atomik kayıt; Windows Credential Manager kodu (cihaz testi bekliyor)
- [x] Gemini Live protokolü ve kesinleşmiş metin toplama testleri
- [ ] Ses kaydı ve iptal/temizlik (Windows, PipeWire)
- [ ] Unicode metin çıkışı (SendInput, KDE kwtype, Hyprland wtype)
- [ ] Tek oturum, Win+Z ve Linux toggle
- [ ] Ortak Rust ayar ekranı
- [ ] Windows tray, OSD, sesli bildirim, autostart ve hata/log ekranı
- [ ] Kurulum ve paketler: Windows, Arch, Fedora, Nix
- [ ] Rust testleri ve Windows/Linux derlemeleri
- [ ] Gerçek Windows mikrofon/Win+Z ve Linux Wayland uçtan uca doğrulama
- [ ] Özellik eşitliği sağlandıktan sonra Python kodunu kaldırma

## Devam kuralı
Tamamlanan işleri yalnızca doğrulama kanıtıyla işaretle. Çalışma notları ve bir sonraki adım `RUST_MIGRATION.md` içinde tutulur. Fallback yok; eski API anahtarı taşıma eklenmez. Win+Z korunur. Tamamlanmamış kod üretim kurulumuna geçirilmez.

## Uygulanan Rust kodu (2026-09-08)
`rust/` bağımsız Cargo uygulamasıdır; Python çağırmaz. Çalışma girişleri: `setup`, `doctor`, `record`, `daemon`; Linux'ta ayrıca `toggle`. Windows'ta konfigürasyon varsa parametresiz açılış daemon başlatır.

- `config.rs`: katı JSON doğrulama, özel kelimeler, atomik kayıt, Linux 0600 izin, Windows Credential Manager (aynı hedef/UTF-8 sözleşmesi).
- `protocol.rs`, `live.rs`: Gemini modeli ve mevcut wire sözleşmesi, PCM16/16 kHz, activityStart/activityEnd, final/interim ayrımı, final settle, bağlantı timeout ve credential-safe hata mesajları.
- `audio.rs`, `session.rs`: PipeWire / WASAPI (cpal), kanal ve örnekleme dönüşümü, sınırlı ses kuyruğu, hata varsa çıktı engelleme, recorder drain/cleanup.
- `output.rs`: KDE kwtype / Hyprland wtype seçimi; Windows UTF-16 SendInput ve fiziksel modifier bırakılmasını bekleme. Fallback yok.
- `controller.rs`: kayıt ve finalizasyonu tek sahip yönetir; finalizasyon sırasında tekrar kayıt açılmaz.
- `windows.rs`: tek daemon mutex, yerel Win+Z hook, tekrar basılı tutmayı engelleme, tray menüsü ve bildirimler.
- `linux.rs`: 0700 runtime dizini, tek daemon kilidi, peer UID doğrulamalı Unix socket üzerinden toggle.
- `ui.rs`: ortak egui ayar ekranı; anahtarı Live API ile doğruladıktan sonra kaydeder.
- `cue.rs`: isteğe bağlı başlangıç/bitiş sesi; varsayılan kapalı.

## Doğrulama
- Linux `cargo test --locked`: 11 test geçti (son değişikliklerden sonra yeniden çalıştırılacak).
- Linux ve Windows GNU hedefi `cargo clippy --all-targets -- -D warnings`: ilk çekirdek geçti; son eklemeler tekrar kontrol ediliyor.
- Gerçek Windows çalıştırma, mikrofon, Win+Z, tray ve Wayland testi yapılmadı.
- API anahtarı olmadığından gerçek Gemini oturumu denenmedi.
- GitHub native Windows/Linux CI sonucu henüz bekleniyor.

## Kalan işler / sonraki adım
1. CI sonuçlarını kontrol et, hata varsa aynı dalda düzelt. Windows MSVC gerçek derlemesi şart.
2. Uçtan uca mock WebSocket ve lifecycle testlerini genişlet; hızlı start/stop ve recorder hata durumlarını doğrula.
3. Windows OSD, autostart, uygulama ikonu, log/doctor ekranı, mikrofon seçim listesini tamamla. Tray Explorer yeniden başladığında tekrar eklenmeli.
4. Ayar ekranını grafik ortamında görüntüleyip doğrula.
5. Windows/Arch/Fedora/Nix kurulum ve paketlerini Rust'a geçir; Linux daemon autostart ve shortcut komutlarını bağla.
6. Gerçek cihaz kabul testlerini yap; sonra Python kaynaklarını ve bağımlılıklarını kaldır.

## Devam komutları
```sh
cd rust
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked
cargo run -- setup
cargo run -- daemon
# Linux'ta başka terminalden / masaüstü Win+Z kısayolundan:
cargo run -- toggle
```

**Henüz üretime hazır değil.** Python kurulumları değiştirilmedi. PR draft kalmalı; üstteki ana özellik eşitliği maddeleri yalnızca tamamı doğrulanınca işaretlenmeli.
