## Amaç
Python uygulamasını Windows ve Linux için Rust ile yeniden yazmak. Bu PR tamamlanana kadar draft kalır.

## Kontrol listesi
- [x] main @ 24c872e mevcut davranış ve platform envanteri
- [ ] Rust proje yapısı, kilitli bağımlılıklar, CI
- [ ] Ayar doğrulama ve atomik kayıt; Windows Credential Manager
- [ ] Gemini Live protokolü ve kesinleşmiş metin toplama testleri
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
