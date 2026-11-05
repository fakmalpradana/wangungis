# ADR-009: Impor point cloud

- Status: **Accepted** 2026-11-05 (disetujui pemilik lewat rencana)

## Keputusan
- Impor LAS memakai crate `las` 0.11, berlisensi MIT dan ditulis dalam Rust murni.
- WangunGIS menulis cache sendiri: grid persegi pada bidang XY, posisi `u16` relatif terhadap batas chunk, warna RGBA, dan tabel JSON di akhir berkas.
- Titik setiap chunk diacak secara deterministik. Viewer dapat membaca prefix kecil sebagai sampel yang tersebar tanpa membangun tingkat octree terpisah.

## Alternatif ditolak
- **PDAL**: dukungan format dan pemrosesannya luas, tetapi ia berbasis C++ dan berat untuk dikirim konsisten di macOS, Windows, dan Linux. ADR-007 sudah memilih crate native sampai v0.5; PDAL tetap dapat diakses kemudian melalui proses Python.
- **Octree**: beberapa tingkat data dan indeks menambah ukuran cache serta kerumitan impor. Grid yang diacak dan dibaca sebagai prefix jauh lebih kecil untuk kebutuhan tampilan awal.

## Konsekuensi dan batas
- Cache cepat dibaca per area dan tingkat detail tanpa dependensi runtime tambahan.
- LAZ belum didukung. Impor tidak mereproyeksi koordinat dan tidak menyimpan klasifikasi.
- Batas grid berasal dari header LAS. Satu pencilan yang sangat jauh di dalam batas header dapat membuat grid terlalu kasar untuk kumpulan titik utama.
