# Rust geçişi — PR #8

Dal: `rewrite/rust`. Amaç: Windows ve Linux uygulaması ile bütün kurulum/paket yollarını Rust'a geçirmek. PR gerçek cihaz kabul testleri bitene kadar draft kalır.

## Yan senaryo incelemesi — 9 Eylül
- [x] Daemon çıkışını normal durdurmadan ayır: kayıt/finalizasyon iptalinde metin yazma, mikrofon görevini bekleyerek temizle
- [x] Tamamlanmış kayıt görevinin iptalde ikinci kez poll edilmesini önle; kayıt sırasında ve sonrasında iptal testleri
- [x] Windows komut kuyruğu doluyken çıkış isteğini kaybetme; worker join öncesinde Quit teslimini garantile
- [x] Win önce bırakıldığında basılı Z'nin tekrarlarını yut; tuş dizisi regresyon testi ekle
- [x] Aynı mesajdaki final + interim metinde geçici kuyruğu koru; eksik metin çıktısını engelleyen test
- [x] Ayar dosyası yazılamazsa Windows anahtarını geri al; ilk anahtarın kaldırılması ve eski anahtarın korunması testleri
- [x] Kaydetme sürerken ayar penceresinin kapanıp işlemi yarıda kesmesini engelle
- [x] Rust 1.95 ile yerelde 28 test, Clippy (`-D warnings`), fmt ve uygulama self-test
- [ ] Yeni değişikliklerin Windows/Linux ve Arch/Fedora/Nix CI kontrolleri (sonuçlar PR açıklamasında güncellenecek)

## Arayüz ve anahtar doğrulama düzeltmesi
- [x] Tamamen İngilizce, bölümlere ayrılmış ayar ekranı ve okunabilir metin/boşluk düzeni
- [x] Smart ve Verbatim için açıklamalar; arayüz dili ile konuşma dili ayrımı
- [x] Sabit kaydetme alanı, görünür bağlantı durumu ve hata mesajı
- [x] Her kaydetmede Gemini doğrulaması; değişmeyen anahtarda doğrulamayı atlama kaldırıldı
- [x] Başarısız doğrulamada eski ayarları koruma ve ilk kurulumda dosya oluşturmama testleri
- [x] Geçersiz setupComplete yanıtı başarı sayılmıyor
- [x] Yerelde 23 test; `12345` ile canlı bağlantı denemesi başarısız oldu
- [x] Yeni arayüz ekran görüntüsü görsel kontrolü ve Windows/Linux CI kontrolleri (`fd21605`)

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
- [x] Yerelde 23 Rust testi (5 localhost WebSocket testi, 3 oturum yaşam döngüsü testi dahil)
- [x] Linux debug build ve `--self-test`
- [x] Shell/PKGBUILD sözdizimi ve diff whitespace kontrolü
- [x] Windows native MSVC derleme, test, installer testleri ve EXE smoke testi (`fd21605`)
- [x] Linux native derleme ve ayar ekranı render/görsel kontrolü (`fd21605`)
- [x] Arch paket derleme, test, kurulum ve self-test (`fd21605`)
- [x] Fedora RPM derleme, test, kurulum ve self-test (`fd21605`)
- [x] Nix KDE/Hyprland paket derlemeleri ve kurulu uygulama self-test (`fd21605`)
- [ ] Gerçek Windows mikrofon/Win+Z/SendInput/tray/OSD testi
- [ ] Gerçek Linux KDE/Hyprland Wayland uçtan uca test
- [ ] Gerçek Gemini API oturumu (bu ortamda API anahtarı yok)

## Başarılı CI koşuları
Bu incelemeden önce doğrulanan uygulama/paket commit'i: `fd21605efcf71cee85a7baa22cab0f8e71ea96d9`. Aşağıdaki bağlantılar önceki sürümün kanıtlarıdır; yeni inceleme commit'inin sonuçları PR açıklamasında ayrıca izlenir.

| Kontrol | Kanıt |
| --- | --- |
| Windows EXE, installer testleri, checksum ve self-test | [Başarılı koşu](https://github.com/Yakrel/ai-dikte/actions/runs/34357509549) |
| Linux/Windows native lint, test, build; Linux GUI render | [Başarılı koşu](https://github.com/Yakrel/ai-dikte/actions/runs/34357509538) |
| Arch uygulama ve KWtype paketleri | [Başarılı koşu](https://github.com/Yakrel/ai-dikte/actions/runs/34357509619) |
| Fedora uygulama ve KWtype RPM'leri | [Başarılı koşu](https://github.com/Yakrel/ai-dikte/actions/runs/34357509547) |
| Nix KDE ve Hyprland paketleri | [Başarılı koşu](https://github.com/Yakrel/ai-dikte/actions/runs/34357509577) |

Windows test paketi: [ai-dikte-windows ZIP](https://github.com/Yakrel/ai-dikte/actions/runs/34357509549/artifacts/10106583231). ZIP içindeki EXE ve SHA256 dosyasını aynı klasöre çıkarın; PR dalındaki `install.ps1 -ArtifactDirectory <klasör>` ile kurun. Artifact saklama süresi 7 gün; daha sonra Build Windows Executable workflow'u `rewrite/rust` dalında yeniden çalıştırılabilir.

## Mevcut doğrulama notları
- `fd21605`: İngilizce arayüz, her kaydetmede doğrulama, yerleşim ve bütün paket kontrolleri başarılı. Geçersiz anahtarın ilk kaydını mevcut sürümde birebir yeniden üretemedik; aynı anahtarda doğrulama atlaması kaldırıldı ve başarısız doğrulamada ayar dosyasının değişmediği test edildi.
- Linux GUI için eksik XKB X11 kütüphanesi eklendi; CI ekran görüntüsü alındı ve görsel olarak kontrol edildi.
- `82d0167`: Windows release EXE, checksum, installer hata testleri ve self-test geçti; Linux release ve GUI smoke testi geçti.
- Nix sabitlenmiş Rust 1.95 kullanıyor. Minimum sürüm 1.95 olarak düzeltildi; yerelde bu sürümle 20 test geçti.
- Arch makepkg alt klasör kaynaklarını basename ile arıyor. Checksum doğrulamasını koruyarak açık yerel file URL kaynaklarına geçildi; kaynak checksum doğrulaması CI'da geçti. Üst klasördeki geçici Cargo.toml ile çakışmayı önlemek için bağımsız `[workspace]` eklendi.
- Fedora RPM derlemesi, 16 paket testi ve self-test geçti; kurulumdaki yanlış `wayland-libs` bağımlılığı Fedora'nın `libwayland-client/cursor/egl` paketleriyle düzeltildi. Arch ve Fedora kurulum CI kontrolleri geçti.
- Python dosyaları artık bu dalda yok; main henüz değiştirilmedi ve PR merge edilmedi. Test için Build Windows Executable koşusundaki `ai-dikte-windows` artifact'ini kullanın. `main` installer'ı merge öncesi eski sürümü indirir.
- Nix sandbox socket açmaya izin vermediğinden yalnız `live::tests` orada atlanır; localhost wire testleri Linux/Windows native CI'da çalışır.

## Kabul senaryoları
1. Windows'ta eski daemon'dan çık. Rust EXE'yi aç; ayarları kaydet, kapat, Win+Z başlat/durdur. Tuşu basılı tutma ve Win'i Z'den önce bırakma Snap/Start menüsünü açmamalı; konsol penceresi görünmemeli.
2. Türkçe karakter/emoji/satır sonları, hızlı çift basış, kayıt sırasında aygıt çıkarma; hatada kısmi metin yazılmamalı. Win+Z finalizasyon sürerken yeni kayıt başlatmamalı.
3. 44.1/48 kHz mikrofonlar, farklı odak uygulamaları, yükseltilmiş hedefte açık SendInput hatası.
4. Bildirim açık/kapalı, ses açık/kapalı, Windows başlangıcı ve Explorer yeniden başlatıldıktan sonra tray.
5. KDE yalnız kwtype, Hyprland yalnız wtype; başka backend'e geçilmemeli. İkinci daemon reddedilmeli, servis durduğunda mikrofon bırakılmalı.
6. Kayıt ve finalizasyon sırasında tray Exit / servis stop: mikrofon bırakılmalı ve metin yazılmamalı. Normal Win+Z durdurma metni yazmaya devam etmeli.
7. Ayarları kaydederken pencereyi kapatmayı dene: işlem bitmeden kapanmamalı. Yazılamayan config yolunda önceki anahtar korunmalı.
8. Yanlış anahtar, kota ve bağlantı kopması; anahtar/ses/metin günlüğe yazılmamalı.

## Devam kuralı
Aynı PR ve dalda çalış. Son commit'in CI koşularını kontrol et, başarısız adımların loglarını okuyup düzelt. İşaretleri yalnız doğrulama kanıtıyla güncelle; gerçek cihaz testlerini yapılmış gibi işaretleme. Kod değişince `packaging/update-arch-sources.sh` çalıştır. PR açıklamasını bu listeyle eşzamanlı güncelle. Yeni commit mevcut CI'ı iptal edebileceğinden, sonuç beklerken sadece durum notu için commit atma.
