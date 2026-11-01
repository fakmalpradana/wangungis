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
- Data WangunGIS disimpan di attribute dictionary Hew pada namespace `"wangungis"` (API `attr_set(target, "wangungis", key, value)`). Dokumen lain menyingkatnya menjadi `wangungis.*`.
  - Tingkat dokumen: CRS (EPSG), origin proyek `f64`.
  - Tingkat Object: LOD, kelas CityGML, atribut bangunan.
- Prototipe #5 membuktikan pola ini: nilai round-trip bit-identik, dan atribut bertahan melewati push/pull, undo, redo, serta reload (`research/hew-spike-ext.md`).
- Yang **belum diuji**, dan wajib punya test di F0/v0.1:
  - Atribut setelah boolean yang membelah Object (`push_pull_through`).
  - Save → load → save menghasilkan zip byte-identik (kriteria keluar v0.1).
  - Hew asli membuka dan menyimpan ulang file kita tanpa membuang namespace `wangungis`.
- Celah: semantik per face (WallSurface/RoofSurface/...) belum bisa disimpan, karena Hew belum punya atribut per face.
  - Sementara, semantik diturunkan dari normal face saat ekspor (`|n_z| > 0,9`). Cara ini **hanya berlaku untuk LOD1 di F0/v0.1**.
  - Cara ini salah untuk atap yang lebih curam dari ±26°, overhang/soffit, balkon, dan ClosureSurface. Cara ini juga membuang semantik hasil impor. Jadi bertentangan dengan ekspor tanpa kehilangan (di atas) dan dengan kriteria keluar v0.2 (round-trip 3D BAG).
  - **Sebelum v0.2 wajib ada** penyimpanan semantik per face: lewat PR ke Hew (`EntityRef::Face`) atau patch kecil di fork (ADR-000, #6).
- **Koreksi 2026-11-01 (ADR-004):** satu `.hew` hanya memuat satu tile kernel, yaitu geometri ≤ 1,5 km dari origin-nya.
  - Proyek kota menjadi wadah berisi banyak `.hew` (satu per tile) ditambah manifest proyek: origin, EPSG, dan daftar tile beserta offset `f64`.
  - Bentuk wadahnya (zip atau folder) diputuskan saat tile kernel dibangun.
  - Proyek kecil, satu tile, tetap satu `.hew` biasa yang bisa dibuka Hew.
- Ekstensi file: `.hew` sampai ada alasan untuk berbeda (**asumsi**: file kita tetap bisa dibuka Hew, dan itu menguntungkan).
