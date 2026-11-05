# ADR-008: Jendela berkas bawaan OS lewat `rfd`

- Status: **Accepted** 2026-11-05 (disetujui pemilik lewat rencana)

## Konteks
Buka, simpan, dan ekspor memakai kotak teks path. Pemilik meminta jendela untuk mencari berkas dan folder, dengan format yang jelas. Aplikasi berjalan di macOS, Windows, dan Linux dari satu basis kode.

## Keputusan
Memakai crate `rfd` 0.17, lisensi MIT, fitur bawaan. `rfd` membuka jendela bawaan OS: panel Cocoa di macOS, Common Item Dialog di Windows, xdg-desktop-portal di Linux dengan cadangan zenity. Dipanggil sinkron di akhir frame.

Ukuran menurut `cargo tree` per target, sebelum dikurangi yang sudah ada di tree lewat `eframe`: 5 crate di Windows, 16 di macOS, 30 di Linux.

## Alternatif ditolak
- **Jendela sendiri di egui (`egui-file-dialog` atau tulis sendiri):** tanpa ketergantungan OS dan bisa diuji di smoke, tetapi tidak memakai pintasan folder, pencarian, dan konfirmasi timpa dari OS, dan menambah kode UI.
- **Fitur `gtk3` di `rfd`:** menambah pustaka sistem saat build dan distribusi Linux.

## Konsekuensi
- Isi jendela tidak bisa diuji otomatis. Test memanggil `App::answer_dialog` dengan path; jendela asli diuji manual tiap OS.
- Konfirmasi timpa berkas kini dari OS. Nama yang diubah aplikasi (`.hew` jadi `.wangun`) tidak pernah menimpa, karena OS tidak menanyakannya.
- Di Linux tanpa portal dan zenity, `rfd` menjawab seperti batal. Aplikasi menebak dari jawaban yang kurang dari 150 ms dan menampilkan pesan. Ini belum diuji di Linux sungguhan.
