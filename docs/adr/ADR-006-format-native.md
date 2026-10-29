# ADR-006: Format file native

- Status: Proposed (bergantung ADR-000; checkpoint manusia AGENTS.md §14)

## Keputusan
- Jalur A: perluas `.hew` (zip: manifest JSON + buffer biner, keluaran byte-stable) dengan blok geospasial: CRS (EPSG), origin `f64`, semantik permukaan, LOD per objek.
- Jalur B: format sendiri dengan pola yang sama — zip, `manifest.json` berversi, buffer biner, skema JSON terdokumentasi.
- Kedua jalur: ekspor CityJSON 2.0 **tanpa kehilangan** semantik dan LOD; uji round-trip otomatis.

## Alternatif ditolak
- **CityJSON sebagai format native**: tidak menyimpan state editor (guide, scenes, styles, riwayat), tetapi tetap jadi format ekspor kelas satu.
- **SQLite**: lebih sulit di-diff dan direview untuk satu orang (asumsi).
