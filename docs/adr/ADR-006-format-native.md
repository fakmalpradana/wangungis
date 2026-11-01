# ADR-006: Format file native

- Status: **Accepted** 2026-11-01 untuk A2 (K2, ADR-000; checkpoint §14 didelegasikan manusia ke agen pada K2)

## Keputusan
- Jalur A: perluas `.hew` (zip: manifest JSON + buffer biner, keluaran byte-stable) dengan blok geospasial: CRS (EPSG), origin `f64`, semantik permukaan, LOD per objek.
- Jalur B: format sendiri dengan pola yang sama — zip, `manifest.json` berversi, buffer biner, skema JSON terdokumentasi.
- Kedua jalur: ekspor CityJSON 2.0 **tanpa kehilangan** semantik dan LOD; uji round-trip otomatis.

## Alternatif ditolak
- **CityJSON sebagai format native**: tidak menyimpan state editor (guide, scenes, styles, riwayat), tetapi tetap jadi format ekspor kelas satu.
- **SQLite**: lebih sulit di-diff dan direview untuk satu orang (asumsi).

## Keputusan untuk A2 (2026-11-01)
- Wadah: `.hew` milik `kernel` Hew apa adanya (zip, manifest berversi, keluaran byte-stable). Kita tidak menulis serializer sendiri.
- Data WangunGIS disimpan di attribute dictionary Hew dengan prefiks `wangungis.`:
  - Tingkat dokumen: CRS (EPSG), origin proyek `f64`.
  - Tingkat Object: LOD, kelas CityGML, atribut bangunan.
- Prototipe #5 membuktikan pola ini: round-trip bit-identik, atribut bertahan melewati push/pull, undo, redo, dan reload (`research/hew-spike-ext.md`).
- Celah: semantik per face (WallSurface/RoofSurface) belum bisa, karena Hew belum punya atribut per face. Sementara itu, semantik diturunkan dari normal face saat ekspor CityJSON. Atribut per face lewat PR ke Hew atau patch kecil di fork (ADR-000, #6).
- Ekstensi file: `.hew` sampai ada alasan untuk berbeda (**asumsi**: file kita tetap bisa dibuka Hew, dan itu menguntungkan).
