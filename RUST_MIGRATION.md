## Amaç
Python uygulamasını Windows ve Linux için Rust ile yeniden yazmak. Bu PR tamamlanana kadar draft kalır.

## Kontrol listesi
- [x] main @ 24c872e mevcut davranış ve platform envanteri
- [x] Rust proje yapısı, kilitli bağımlılıklar, CI tanımı
- [x] Ayar doğrulama ve atomik kayıt; Windows Credential Manager kodu (cihaz testi bekliyor)
- [x] Gemini Live protokolü ve kesinleşmiş metin toplama testleri
- [x] Ses kaydı ve iptal/temizlik kodu (Windows, PipeWire); cihaz kabul testi aşağıda ayrı
- [x] Unicode metin çıkışı kodu (SendInput, KDE kwtype, Hyprland wtype)
- [x] Tek oturum, Win+Z ve Linux toggle kodu
- [x] Ortak Rust ayar ekranı kodu; görsel doğrulama bekliyor
- [x] Windows tray, OSD, sesli bildirim, autostart ve hata/log ekranı kodu; gerçek Windows testi bekliyor
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
- `diagnostics.rs`: boyutu sınırlı oturum günlüğü ve anahtarı göstermeyen tanılama raporu. Windows tray üzerinden günlük/tanılama penceresi açılır.
- Ayar ekranında yalnızca API tercihleri değişirse bağlantı doğrulanır; yerel bildirim/ses tercihleri çevrimdışı kaydedilebilir.
- Mevcut Arch PKGBUILD içindeki main kaynaklı iki eski SHA256 düzeltildi; Python dosyaları değiştirilmedi.

## Doğrulama
- Linux `cargo test --locked`: 15 test geçti; dördü gerçek localhost WebSocket mock sunucusuyla çalışıyor.
- Linux ve Windows GNU hedefi `cargo clippy --all-targets -- -D warnings` geçti. Linux debug executable derlendi.
- Gerçek Windows çalıştırma, mikrofon, Win+Z, tray ve Wayland testi yapılmadı.
- API anahtarı olmadığından gerçek Gemini oturumu denenmedi.
- İlk kod checkpoint (80182cb): GitHub Windows MSVC ve Linux test/lint/release derlemeleri başarılı: https://github.com/Yakrel/ai-dikte/actions/runs/34268923051
- Son kod checkpoint (129677b): Windows/Linux lint geçti; test, GUI render ve release build devam ediyor: https://github.com/Yakrel/ai-dikte/actions/runs/34269569112
- Yerelde Xvfb yok; paket kurulumu ortam izinleriyle engellendi. Linux CI içine Xvfb ayar ekranı render smoke testi eklendi.

## Kalan işler / sonraki adım
1. CI sonuçlarını kontrol et, hata varsa aynı dalda düzelt. Windows MSVC gerçek derlemesi şart.
2. Uçtan uca mock WebSocket ve lifecycle testlerini genişlet; hızlı start/stop ve recorder hata durumlarını doğrula.
3. Uygulama ikonunu executable/tray içine göm, mikrofon numarası yerine aygıt seçim listesini tamamla. OSD/autostart/log/doctor ve Explorer restart desteği yazıldı; gerçek Windows testi bekliyor.
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

## Kabul testi senaryoları
- Windows: önce eski Python daemon'dan çık; aynı mutex ikinci daemon'ı engeller. Rust EXE ile setup, sonra daemon. Win+Z başlat/durdur, tuşu basılı tut, Win'i Z'den önce bırak, hızlı çift basış. Başlat menüsü/Snap paneli veya konsol penceresi açılmamalı.
- Windows: Türkçe karakterler, emoji ve satır sonları; farklı giriş hızları (44.1/48 kHz), mikrofon çıkarma, yükseltilmiş hedefte açık SendInput hatası, Explorer restart, bildirim kapalı/açık, başlangıç ve ses seçenekleri.
- Linux: PipeWire varsayılan kaynağı; KDE yalnız kwtype, Hyprland yalnız wtype. Daemon açıkken ikinci daemon açık hata vermeli. Toggle sonrası metin odaktaki alana yazılmalı.
- API: yanlış anahtar, kota hatası, bağlantı kopması ve sessiz mikrofon. Hatalarda kısmi metin veya alternatif backend kullanılmamalı; API anahtarı günlüğe yazılmamalı.
- Rust paketleri ve Python kaldırma henüz yapılmadı; mevcut installer kullanılırsa hâlâ Python sürümü kurulur.

## Sonraki oturumun ilk işi
PR #8 / `rewrite/rust` dalında devam et. Önce 129677b commitinin Rust CI sonucuna bak; sonradan gelen yalnız dokümantasyon commitinin yeni Rust koşusu olmaması normaldir. `RUST_MIGRATION.md` listesini ilerledikçe ve PR açıklamasını aynı anda güncelle. En yeni CI artifact'i yerine eski Python workflow artifact'ini yanlışlıkla test etme: Rust workflow adı `Rust rewrite`.
