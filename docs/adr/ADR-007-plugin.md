# ADR-007: Plugin dan scripting

- Status: Proposed

## Keputusan
- API Python, dijalankan sebagai **proses terpisah** dengan API terdokumentasi (pendekatan yang juga direncanakan Hew). Embedding PyO3 hanya bila latensi proses terbukti terlalu tinggi.
- IfcOpenShell, GDAL penuh, dan PDAL diakses dari sisi Python.
- **Tidak meniru Ruby API SketchUp.** Google v. Oracle (2021) menyatakan reimplementasi API Java sebagai fair use, tetapi putusannya sempit dan bergantung fakta ([LWN](https://lwn.net/Articles/851617)). Manfaatnya kecil untuk USP LOD. Ini bukan nasihat hukum.

## Alternatif ditolak
- **WASM plugin**: sandbox bagus, tetapi ekosistem geospasial Python jauh lebih besar; dipertimbangkan setelah v0.5.
- **Embedding Python in-process sebagai default**: crash plugin menjatuhkan aplikasi; packaging runtime Python lebih rumit tanpa signing.
