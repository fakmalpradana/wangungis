# ADR-007: Plugin dan scripting

- Status: **Accepted** 2026-11-01 (tidak bergantung jalur; dikonfirmasi saat K2)

## Keputusan
- API Python, dijalankan sebagai **proses terpisah** dengan API terdokumentasi (pendekatan yang juga direncanakan Hew). Embedding PyO3 hanya bila latensi proses terbukti terlalu tinggi.
- IfcOpenShell, GDAL penuh, dan PDAL diakses dari sisi Python, mulai v0.5 bersama API Python (#95).
- v0.2–v0.4 memakai crate Rust native di `io` (`shapefile`, `tiff`, `las`). `lod` menerima data yang sudah diparse dari `io` dan tidak melakukan I/O sendiri.
- **Tidak meniru Ruby API SketchUp.** Google v. Oracle (2021) menyatakan reimplementasi API Java sebagai fair use, tetapi putusannya sempit dan bergantung fakta ([LWN](https://lwn.net/Articles/851617)). Manfaatnya kecil untuk USP LOD. Ini bukan nasihat hukum.

## Alternatif ditolak
- **WASM plugin**: sandbox bagus, tetapi ekosistem geospasial Python jauh lebih besar; dipertimbangkan setelah v0.5.
- **Embedding Python in-process sebagai default**: crash plugin menjatuhkan aplikasi; packaging runtime Python lebih rumit tanpa signing.
