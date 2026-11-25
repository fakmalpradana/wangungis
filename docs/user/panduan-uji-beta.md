# Panduan uji beta WangunGIS di Windows

Untuk 3D engineer yang menguji WangunGIS. Aplikasi ini masih pra-rilis, jadi simpan salinan data penting dan jangan memakainya untuk pekerjaan produksi.

Tujuan uji ini ada tiga: menemukan masalah yang hanya muncul di Windows, menilai apakah aplikasi bisa dipakai membangun model 3D dari point cloud, dan mengumpulkan laporan yang cukup jelas untuk diperbaiki. Laporan ditulis memakai [format umpan balik](format-umpan-balik.md).

## 1. Memasang

Pilih satu jalur. Jalur A lebih cepat bila pemilik proyek sudah membagikan file zip.

Catatan untuk pemilik proyek: file zip dibuat oleh workflow `binaries` di GitHub Actions. Buka tab Actions, pilih `binaries`, lalu **Run workflow** pada `main`. Setelah selesai, unduh artifact `wangungis-<commit>-windows-x86_64` dari halaman run tersebut (perlu login GitHub). Artifact itu berupa zip yang berisi zip aplikasi beserta file `.sha256`, jadi bagikan zip di dalamnya.

### Jalur A: memakai file zip dari pemilik proyek

1. Ekstrak `wangungis-<commit>-windows-x86_64.zip` ke folder biasa, misalnya `C:\WangunGIS`. Kode `<commit>` pada nama file adalah versi yang Anda uji, jadi catat kodenya.
2. Buka PowerShell, lalu jalankan:

   ```powershell
   cd C:\WangunGIS
   $env:RUST_BACKTRACE = 1
   .\wangungis.exe
   ```

Aplikasi ini belum ditandatangani secara digital, jadi Windows bisa menampilkan "Windows protected your PC". Bila file berasal langsung dari pemilik proyek, pilih **More info** lalu **Run anyway**. Bila muncul galat tentang `VCRUNTIME140.dll`, pasang Microsoft Visual C++ Redistributable x64 dari situs Microsoft.

### Jalur B: membangun dari kode sumber

Sediakan ruang kosong sekitar 10 GB (perkiraan) dan koneksi internet. Build pertama bisa lama. Build berikutnya jauh lebih cepat.

1. Pasang [Git for Windows](https://git-scm.com/download/win).
2. Unduh dan jalankan `rustup-init.exe` dari [rustup.rs](https://rustup.rs). Bila diminta memasang prasyarat Visual Studio, pilih pemasangan cepat. Lalu pilih pemasangan standar, supaya toolchain `x86_64-pc-windows-msvc` yang terpasang.
3. Tutup PowerShell, lalu buka yang baru supaya `cargo` dikenali.
4. Dari folder hasil clone, jalankan:

   ```powershell
   git pull
   git rev-parse --short HEAD
   cargo build --release -p wangungis --locked
   ```

   Keluaran `git rev-parse` adalah kode versi untuk laporan Anda. Versi Rust yang tepat, 1.97.1, terpasang otomatis dari `rust-toolchain.toml`.
5. Jalankan aplikasi:

   ```powershell
   $env:RUST_BACKTRACE = 1
   .\target\release\wangungis.exe
   ```

## 2. Membuka aplikasi

Selalu jalankan dari PowerShell, bukan klik dua kali. Bila aplikasi crash, pesan galatnya tertulis di PowerShell dan pesan itulah yang paling membantu perbaikan. Jangan tutup jendela PowerShell selama menguji.

Untuk langsung membuka proyek, tambahkan path file:

```powershell
.\wangungis.exe "C:\Data Uji\gedung.hew"
```

Tombol di Windows: semua `Cmd` pada panduan langkah berarti `Ctrl`. Orbit memakai seret tombol tengah mouse, atau Alt + seret kiri. Daftar lengkap ada di bagian Kontrol pada [panduan langkah](uji-asap.md#kontrol).

Data aplikasi tersimpan di `%USERPROFILE%\.wangungis\`: `autosave.bin` untuk pemulihan setelah crash dan `cache\` untuk cache point cloud. Cache memakai sekitar 35% ukuran file LAS dan belum dibersihkan otomatis.

## 3. Urutan uji

Kerjakan berurutan. Bila waktu terbatas, A dan B paling penting.

### A. Pemeriksaan khusus Windows (perkiraan 20 menit)

Bagian ini belum pernah diuji manusia di Windows. CI hanya membuktikan aplikasi bisa dibangun dan test otomatisnya lulus.

- [ ] Aplikasi terbuka. Garis sumbu tampil dan orbit, pan, serta zoom berjalan dengan mouse Anda.
- [ ] **Jendela berkas.** Berkas > Buka, Simpan sebagai, Ekspor, dan Impor point cloud membuka jendela Windows, dan filter jenis filenya benar.
- [ ] **Path sulit.** Simpan dan buka lagi proyek di folder yang namanya memuat spasi dan huruf non-Inggris. Ulangi di drive selain `C:` bila ada.
- [ ] **Ctrl.** Ctrl+Z, Ctrl+Shift+Z, Ctrl+S, Ctrl+O, dan Ctrl+Shift+S berjalan. Tap Ctrl saat Pindah menulis "Menyalin", dan tap Ctrl saat Push/Pull menulis "Solid baru".
- [ ] **Skala tampilan.** Pada Settings > System > Display > Scale, coba 100%, lalu 125% atau 150%. Titik dan snap harus tetap tepat di bawah kursor.
- [ ] **Dua monitor**, bila ada. Pindahkan jendela antar-monitor, lalu periksa lagi ketepatan klik.
- [ ] **Pemulihan crash.** Buat kotak, jangan disimpan, tunggu lebih dari 60 detik, lalu tutup paksa `wangungis.exe` lewat Task Manager. Buka lagi: aplikasi menawarkan pemulihan dan kotaknya kembali.

### B. Tugas nyata: gedung dari point cloud (perkiraan 45 menit)

Pakai file LAS dari pekerjaan Anda, sebaiknya besar. Catat waktu dan bagian yang membuat Anda berhenti atau bingung.

1. **Berkas > Impor point cloud**, pilih `.las`. Catat lama impor pertama dan impor kedua. Catat juga angka `ms/frame` di kanan bawah saat orbit terus-menerus. Pada monitor 60 Hz, 16,7 ms berarti aplikasi mengikuti monitor dan itu wajar.
2. Pilih satu gedung. Tekan **K** dan potong awan di sekitarnya. Atur ukuran titik di panel.
3. Dari tampilan **Atas**, gambar tapak gedung dengan snap ke titik awan. Lalu pakai **P** untuk menaikkannya sampai titik atap.
4. Tambahkan detail yang biasa Anda buat: atap, bukaan, cat, grup, dan tag.
5. Bandingkan model dengan awan dari beberapa sudut. Ukur beberapa jarak dengan **T** dan bandingkan dengan nilai yang Anda harapkan. Salah ukur sekecil apa pun dilaporkan sebagai temuan presisi.
6. Simpan, tutup aplikasi, buka lagi. Model dan awan harus kembali sama.
7. **Berkas > Ekspor** ke `.glb` atau `.obj`, lalu buka di Blender. Periksa skala, posisi, warna, dan arah normal.
8. Tuliskan penilaian Anda: seberapa jauh aplikasi ini bisa menggantikan alur kerja Anda sekarang, dan apa yang paling menghalangi.

### C. Langkah rinci

Kerjakan langkah 1 sampai 36 di [panduan langkah](uji-asap.md#langkah). Langkah 21 boleh dilewati bila tidak punya model kota. Langkah yang menulis "nilai" meminta pendapat Anda, jadi tuliskan pendapatnya walau fiturnya berjalan.

## 4. Melapor

Pakai [format umpan balik](format-umpan-balik.md): satu ringkasan per sesi uji, dan satu blok untuk setiap temuan. Kirim ke pemilik proyek.

- Satu masalah, satu temuan. Jangan digabung walau terjadi di sesi yang sama.
- Tuliskan langkah sampai orang lain bisa mengulanginya. Temuan yang bisa diulang paling cepat diperbaiki. Temuan yang hanya terjadi sekali tetap dilaporkan, dengan keterangan itu.
- Kebingungan juga temuan. Bila Anda ragu harus menekan apa, laporkan, walau akhirnya berhasil.
- Sebelum melapor, periksa bagian [Yang diketahui belum ada](uji-asap.md#yang-diketahui-belum-ada). Bila batas itu menghalangi pekerjaan Anda, laporkan sebagai permintaan dan jelaskan dampaknya.
- **Data klien tidak boleh ikut ke tempat publik.** Repositori ini publik. Jangan mengunggah file LAS, proyek, koordinat, atau tangkapan layar milik klien ke GitHub tanpa izin pemilik data.

## 5. Masalah umum

| Gejala | Yang dicoba |
|---|---|
| `cargo` tidak dikenali | Tutup PowerShell, buka yang baru |
| `link.exe not found` | Pasang prasyarat Visual Studio C++ lewat Visual Studio Installer, lalu build lagi |
| `the lock file needs to be updated` | Jalankan `git status`. Bila `Cargo.lock` berubah, laporkan ke pemilik proyek. Jangan menghapusnya |
| Toolchain 1.97.1 tidak terpasang otomatis | Jalankan `rustup toolchain install` dari folder repo |
| Jendela hitam atau kosong, atau galat GPU saat dibuka | Perbarui driver GPU. Jangan menguji lewat Remote Desktop atau mesin virtual, lalu laporkan pesan PowerShell |
| Gambar patah pada laptop dengan dua GPU | Pada Settings > System > Display > Graphics, atur `wangungis.exe` ke High performance, lalu laporkan hasilnya |
