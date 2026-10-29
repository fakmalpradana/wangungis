# Pra-evaluasi titik ekstensi Hew (kriteria #5)

Tanggal: 2026-10-29. Hew v1.3.1 (`612c1c1`). Prototipe: crate di luar pohon Hew (`~/Desktop/3_dummy/wangungis-spikes/s5-hew-ext`) yang bergantung pada `kernel` lewat path dependency. Sumber: [`hew-spike-ext.rs`](hew-spike-ext.rs). **Tidak ada satu baris pun kode Hew yang diubah.**

## Yang dibuktikan prototipe
1. **CRS dan origin proyek** disimpan di *attribute dictionary* dokumen Hew (`attr_set(AttrTarget::Document, "wangungis", …)`). Setelah save → `Document::load`, nilai origin `[430000.123, 9150000.456, 0]` kembali **bit-identik**. Menurut dokumentasi Hew, attribute dictionary adalah "the persistence extension point for API clients, plugins, and overlays" (`kernel/src/attr.rs`). Kernel menyimpan isinya tanpa pernah menafsirkannya.
2. **Atribut per bangunan** (`function`) disimpan pada Object (`EntityRef::Object`) dan selamat setelah push/pull dan reload.
3. **Ekspor CityJSON 2.0 LOD1** dibaca langsung dari face Object Hew.
   - Vertex berupa bilangan bulat mm relatif terhadap origin, dengan `transform.translate` = origin. Jadi koordinat UTM absolut tidak pernah masuk ke kernel (sesuai temuan S2).
   - Semantik permukaan (Roof/Ground/Wall) diturunkan dari normal face.
   - **Hasil validasi: val3dity 2/2 Building valid, cjval valid.** Orientasi face Hew sudah sesuai konvensi CityJSON tanpa perlu dikoreksi.

## Celah yang ditemukan
| Kebutuhan | Status di Hew | Konsekuensi |
|---|---|---|
| Atribut per **face** (override semantik manual, mis. ClosureSurface) | Tidak ada. `EntityRef` hanya mencakup Object, Sketch, Group, Component, Instance, Guide, Material, Tag, Annotation | Butuh perubahan kernel (PR upstream). Tanpa itu, semantik hanya bisa diturunkan dari normal, atau "dititipkan" lewat material per face (cara akal-akalan) |
| Fitur LOD dipanggil dari UI | Kode kita harus melewati `wasm-api` dan `app/` milik Hew | Butuh fork atau PR upstream. Ini bergantung pada kriteria #6 |
| Transformasi CRS (PROJ) | Kernel WASM tidak bisa memuat PROJ (C/C++) | Jalankan di sisi native Tauri, atau pakai port Rust murni (mis. `proj4rs`, **asumsi**, belum dicek) |

## Penilaian sementara
**#5 lulus sebagian.**
- Lulus untuk sisi data dan kernel: CRS, origin, atribut bangunan, dan ekspor CityJSON yang valid bisa dibuat sebagai crate terpisah tanpa mengubah kernel.
- Belum lulus untuk atribut per face dan integrasi UI. Keduanya butuh PR yang diterima pengelola (#6) atau fork permanen.
