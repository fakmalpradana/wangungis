# 2026-11-05 — Tooltip snap dan inference paralel (#25, #24)

Tujuan: PR 2 dari delapan fitur wajib v0.1.0.

Selesai: label dan warna untuk semua `SnapKind`, tooltip dekat kursor, `Chain::target` dan `Chain::infer`, pratinjau merah muda.

Temuan: Hew hanya mendeklarasikan `Parallel` dan `Perpendicular`, tanpa pernah menghasilkannya. Kolom Hew di matrix dikoreksi menjadi P.

Keputusan:
- Hanya Garis di v0.1.
- Sudut memakai `SOFT_AXIS_APERTURE` Hew, jadi tidak ada toleransi baru.
- Hanya menggantikan titik bebas dan `OnFace`. Peringkat Hew menaruh dua jenis ini paling bawah, tetapi di bawah `OnFace` inferensi tidak akan pernah muncul di atas face.

Belum terbukti otomatis: tampilan tooltip di layar. Test hanya memeriksa labelnya. Pemilik menilai saat uji.

Langkah berikut: PR 3, Push/Pull Ctrl.
