# Format umpan balik uji beta

Salin bagian yang perlu ke chat, email, atau dokumen, lalu isi. Teks biasa sudah cukup. Isi `-` bila tidak relevan, dan jangan menebak: tulis "tidak tahu" bila memang tidak tahu.

## Ringkasan sesi

Isi satu kali untuk setiap sesi uji.

```text
SESI UJI
Penguji:
Tanggal:
Versi (kode commit):
Cara pasang: zip / build sendiri
Windows (jalankan winver):
CPU dan RAM:
GPU (Task Manager > Performance > GPU):
Monitor dan skala tampilan: mis. 1920x1080, 125%
Mouse: tiga tombol / trackpad
Data uji: nama, ukuran file, jumlah titik, EPSG. Boleh disamarkan
Bagian yang dikerjakan: A / B / C
Langkah C yang dikerjakan: mis. 1-20, 30-36
Langkah C yang gagal: mis. 13, 32

Hasil ukur point cloud
  Impor pertama (detik):
  Impor kedua (detik):
  ms/frame saat orbit, biasa dan terburuk:

Penilaian keseluruhan
  Bisa dipakai membangun model dari point cloud? ya / sebagian / tidak
  Tiga penghalang terbesar:
  Yang sudah terasa baik:
  Seberapa jauh menggantikan alur kerja sekarang, dan alat apa yang dibandingkan:
```

## Temuan

Salin blok ini untuk setiap temuan.

```text
TEMUAN [inisial]-[nomor]
Judul: satu kalimat, gejala bukan dugaan penyebab
Jenis: Bug / Crash / Presisi / Kinerja / Kemudahan pakai / Permintaan
Keparahan: S1 / S2 / S3 / S4
Terulang: selalu / kadang (x dari y kali) / sekali
Langkah panduan terkait: mis. C-13, A-Ctrl, B-3, atau -

Langkah mengulang:
1.
2.
3.

Yang diharapkan:
Yang terjadi:
Nilai terukur, bila soal ukuran atau kinerja: harapan lawan hasil, dengan satuan
Dampak pada pekerjaan, dan jalan lain yang ada:

Lampiran:
  Pesan PowerShell, salin utuh bila crash:
  Tangkapan layar atau video:
  File untuk mengulang, boleh dibagikan? ya / tidak, data klien
```

## Keparahan

| Tingkat | Arti | Contoh |
|---|---|---|
| **S1 Pemblokir** | Crash, aplikasi macet, data hilang atau rusak, atau ukuran geometri salah tanpa peringatan | Aplikasi tertutup saat impor LAS. File yang disimpan tidak bisa dibuka |
| **S2 Berat** | Fungsi utama gagal dan tidak ada jalan lain yang wajar | Jendela Simpan tidak muncul di Windows |
| **S3 Sedang** | Fungsi berjalan tetapi salah atau merepotkan, dan ada jalan lain | Snap sering memilih titik yang salah |
| **S4 Ringan** | Kosmetik, atau usulan perbaikan | Teks tombol kurang jelas |

Ragu antara dua tingkat? Pilih yang lebih tinggi. Pemilik proyek akan meninjaunya.

## Contoh pengisian

Contoh berikut hanya menunjukkan bentuk pengisian dan bukan temuan nyata.

```text
TEMUAN FA-3
Judul: Titik snap bergeser dari kursor pada skala tampilan 150%
Jenis: Presisi
Keparahan: S2
Terulang: selalu
Langkah panduan terkait: A-Skala tampilan

Langkah mengulang:
1. Atur skala Windows ke 150%, lalu buka aplikasi.
2. Tekan R, lalu klik tanah.
3. Dekatkan kursor ke sudut kotak.

Yang diharapkan: penanda muncul tepat di sudut di bawah kursor.
Yang terjadi: penanda muncul sekitar 40 piksel di kiri kursor.
Nilai terukur: -
Dampak pada pekerjaan, dan jalan lain yang ada: sudut sulit diklik. Jalan lain: skala 100%.

Lampiran:
  Pesan PowerShell, salin utuh bila crash: -
  Tangkapan layar atau video: snap-150.mp4
  File untuk mengulang, boleh dibagikan? tidak perlu file
```

## Cara laporan diolah

1. Pemilik proyek mengumpulkan laporan dan menyaring data klien.
2. Setiap temuan menjadi satu issue dengan keparahan yang sudah ditinjau. Temuan yang sama digabung.
3. Perbaikan dimulai dari test otomatis yang mengulang masalah itu, lalu diperbaiki lewat PR.
4. Penguji mendapat kode commit baru dan memeriksa ulang temuannya, lalu menjawab: selesai / masih / berubah.
5. Rilis v0.1.0 menunggu sampai tidak ada S1 dan S2 yang terbuka.
