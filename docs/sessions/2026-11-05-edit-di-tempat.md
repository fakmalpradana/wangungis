# 2026-11-05 — Edit isi grup di tempat (#42)

Tujuan: PR 5 dari delapan fitur wajib v0.1.0.

Selesai: `Scene::{enter, leave, session, editable_face, sketch_editable}`, lingkup edit, simpan dan ekspor seolah sesi tertutup, peredupan, klik dua kali, Esc, tombol Edit isi dan Keluar.

Temuan dari test:
- Hew tidak menjaga `transform_sketch_island`, jadi gambar bebas di luar sesi bisa berubah. Sekarang dijaga di `Scene`.
- Drafting Hew menggabungkan semua sketch bebas pada satu bidang, dan penutupan sesi hanya melipat sketch yang dibuat sesi. Garis di dalam sesi pada bidang yang berisi gambar luar akan berakhir di luar grup. Sekarang ditolak dengan pesan. Berlaku dua arah: garis di luar grup pada bidang yang berisi gambar milik grup juga akan masuk ke grup, jadi ikut ditolak.

Review (Codex dan agy, temuan dicocokkan dengan kode):
- Valid dan diperbaiki: method `Scene` yang hanya memeriksa tile; panduan di tile lain; sketch milik objek; aturan bidang harus sama dengan Hew; grup di dalam komponen; tile lain masih menjawab saat sesi terbuka; klik cepat setelah klik lain membuka grup.
- Ditolak: klaim agy bahwa `group_session_group` membaca frame luar. Kodenya hanya membaca frame terdalam.

Keputusan: isi grup tertutup tidak lagi bisa diedit. Disetujui pemilik lewat rencana.

Batas v0.1: instance saudara hilang saat komponen dibuka; tile lain tidak memudar; tidak bisa menggambar pada bidang yang berisi gambar di sisi lain batas grup.

Langkah berikut: PR 6, Tags.
