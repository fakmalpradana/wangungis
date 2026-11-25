# Panduan uji Windows dan format umpan balik

Tujuan: engineer bisa pasang, buka, uji di Windows, lalu lapor dalam bentuk yang bisa jadi perbaikan.
Selesai: `docs/user/panduan-uji-beta.md`, `docs/user/format-umpan-balik.md`, tautan dan path Windows di `uji-asap.md`.
Keputusan: format teks biasa, bukan issue form GitHub. Repo publik, laporan bisa memuat data klien. Issue dibuat setelah pemilik menyaring.
Keputusan: `uji-asap.md` tetap sumber langkah 1-36, panduan beta tidak menyalinnya.
Keputusan: jalankan lewat PowerShell. `ui` tanpa `windows_subsystem`, jadi pesan panic tampil di konsol. Aplikasi belum punya log file.
Belum terbukti: semua perintah pemasangan belum dicoba di Windows. CI hanya build dan test di `windows-2025`. Zip exe belum dicoba di mesin bersih, ketergantungan VC++ runtime berasal dari perilaku umum Rust MSVC.
Langkah berikut: pemilik build di Windows dan coba bagian A, lalu bagikan ke engineer. Temuan masuk lewat format umpan balik.
