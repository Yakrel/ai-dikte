# Rust geçişi — PR #8

Dal: `rewrite/rust`. Amaç: Windows ve Linux uygulaması ile bütün kurulum/paket yollarını Rust'a geçirmek. PR gerçek cihaz kabul testleri bitene kadar draft kalır.

## Tamamlanan uygulama işleri
- [x] Gemini Live wire sözleşmesi, final/interim ayrımı, tekrar eden final segmentlerini koruma
- [x] Atomik ayar dosyası; Windows Credential Manager, Linux 0600 izin
- [x] Windows WASAPI mikrofon kaydı ve 16 kHz mono PCM dönüşümü
- [x] Linux PipeWire kayıt, sınırlı kuyruk ve durdururken kalan sesin gönderilmesi
- [x] SendInput / KDE kwtype / Hyprland wtype; alternatif backend yok
- [x] Tek daemon, Win+Z hook, Linux UID doğrulamalı kontrol soketi
- [x] Ortak egui ayarlar; mikrofon isimleriyle seçim, eksik/aynı isimli aygıtta açık hata
- [x] Windows EXE/tray ikonu, OSD, ses, autostart, tanılama ve günlük pencereleri
- [x] Windows ilk çalıştırmada setup, kaydettikten sonra daemon; iptal edilirse daemon başlatılmaz
- [x] Installer için ağ gerektirmeyen self-test ve noninteractive check-config
- [x] Linux kullanıcı systemd servisi ve Hyprland kısayol yönetimi
- [x] Windows kurulum/release workflow: Cargo EXE + checksum; PR artifact dizininden açık kurulum seçeneği
- [x] Arch Rust PKGBUILD ve SHA256 yenileme scripti
- [x] Fedora Cargo vendor ile offline Rust RPM tarifi
- [x] Nix buildRustPackage ve platform backend/library wrapper'ları
- [x] Python uygulaması, bağımlılıkları ve eski Python testleri kaldırıldı
- [x] README Rust kurulumları ve PR test adımlarıyla güncellendi

## Doğrulama
- [x] Yerelde 20 Rust testi (4 localhost WebSocket testi, 3 oturum yaşam döngüsü testi dahil)
- [x] Linux debug build ve `--self-test`
- [x] Shell/PKGBUILD sözdizimi ve diff whitespace kontrolü
- [x] Windows native MSVC derleme, test, installer testleri ve EXE smoke testi (`82d0167`)
- [x] Linux native derleme ve ayar ekranı render/görsel kontrolü (`82d0167`)
- [ ] Arch paket kurulum testi
- [ ] Fedora RPM derleme ve kurulum testi
- [ ] Nix KDE/Hyprland paket derlemeleri
- [ ] Gerçek Windows mikrofon/Win+Z/SendInput/tray/OSD testi
- [ ] Gerçek Linux KDE/Hyprland Wayland uçtan uca test
- [ ] Gerçek Gemini API oturumu (bu ortamda API anahtarı yok)

## Mevcut doğrulama notları
- Linux GUI için eksik XKB X11 kütüphanesi eklendi; CI ekran görüntüsü alındı ve görsel olarak kontrol edildi.
- `82d0167`: Windows release EXE, checksum, installer hata testleri ve self-test geçti; Linux release ve GUI smoke testi geçti.
- Nix sabitlenmiş Rust 1.95 kullanıyor. Minimum sürüm 1.95 olarak düzeltildi; yerelde bu sürümle 20 test geçti.
- Arch makepkg alt klasör kaynaklarını basename ile arıyor. Checksum doğrulamasını koruyarak açık yerel file URL kaynaklarına geçildi; paket CI sonucu bekleniyor.
- Python dosyaları artık bu dalda yok; main henüz değiştirilmedi ve PR merge edilmedi. Test için Build Windows Executable koşusundaki `ai-dikte-windows` artifact'ini kullanın. `main` installer'ı merge öncesi eski sürümü indirir.
- Nix sandbox socket açmaya izin vermediğinden yalnız `live::tests` orada atlanır; localhost wire testleri Linux/Windows native CI'da çalışır.

## Kabul senaryoları
1. Windows'ta eski daemon'dan çık. Rust EXE'yi aç; ayarları kaydet, kapat, Win+Z başlat/durdur. Tuşu basılı tutma ve Win'i Z'den önce bırakma Snap/Start menüsünü açmamalı; konsol penceresi görünmemeli.
2. Türkçe karakter/emoji/satır sonları, hızlı çift basış, kayıt sırasında aygıt çıkarma; hatada kısmi metin yazılmamalı. Win+Z finalizasyon sürerken yeni kayıt başlatmamalı.
3. 44.1/48 kHz mikrofonlar, farklı odak uygulamaları, yükseltilmiş hedefte açık SendInput hatası.
4. Bildirim açık/kapalı, ses açık/kapalı, Windows başlangıcı ve Explorer yeniden başlatıldıktan sonra tray.
5. KDE yalnız kwtype, Hyprland yalnız wtype; başka backend'e geçilmemeli. İkinci daemon reddedilmeli, servis durduğunda mikrofon bırakılmalı.
6. Yanlış anahtar, kota ve bağlantı kopması; anahtar/ses/metin günlüğe yazılmamalı.

## Devam kuralı
Aynı PR ve dalda çalış. Son commit'in CI koşularını kontrol et, başarısız adımların loglarını okuyup düzelt. İşaretleri yalnız doğrulama kanıtıyla güncelle; gerçek cihaz testlerini yapılmış gibi işaretleme. Kod değişince `packaging/update-arch-sources.sh` çalıştır. PR açıklamasını bu listeyle eşzamanlı güncelle. Yeni commit mevcut CI'ı iptal edebileceğinden, sonuç beklerken sadece durum notu için commit atma.
