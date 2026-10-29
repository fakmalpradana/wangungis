# Spike S3 — face poligon ↔ Manifold (boolean bukaan jendela)

Tanggal: 2026-10-29. Crate: `manifold-csg` 0.4.1 (Apache-2.0/MIT; mengompilasi manifold3d C++ lewat CMake, build pertama ±30 detik di M4). Sumber test: [`spike-s3-manifold.rs`](spike-s3-manifold.rs), dijalankan di crate terpisah `~/Desktop/3_dummy/wangungis-spikes/s3-manifold`. Perintah: `cargo test --release -- --nocapture`.

## Pertanyaan
Bila model kita memakai face poligon planar, apakah alur berikut tetap menghasilkan face yang planar dalam toleransi kernel, dengan volume yang benar?
1. Triangulasi face poligon.
2. Boolean di Manifold.
3. Hasil segitiga dikelompokkan kembali menjadi face poligon berdasarkan kesamaan bidang.

## Metode
- Bangunan: footprint L (8–30 m per sisi, takik 20–70%) diekstrusi 3–20 m.
- Bukaan jendela (0,6–2 × 0,6–1,5 m) dikurangkan dari dinding selatan sebagai kantong sedalam 0,5 m.
- Seluruh model diputar 0–360° dan digeser ke origin acak.
- Segitiga hasil dikelompokkan menjadi face bila normalnya sama (toleransi sudut 1e-9) dan offset bidangnya sama (toleransi 1e-6 m). Lalu diukur jarak terbesar vertex ke bidang face-nya.

## Hasil

| Kasus | Jumlah | Volume (galat relatif) | Genus | Sliver | Residu planaritas maks |
|---|---|---|---|---|---|
| Lokal (origin ±1 km), proptest | 1.000 kasus acak | < 1e-9, semua lulus | 0 | 0 | ≤ 1e-9 m, semua lulus |
| UTM absolut (430000,123; 9150000,456) | 200 rotasi | 1,4e-10 | 0 | 0 | **1,6e-7 m** |

## Kesimpulan
1. Dengan koordinat lokal, alur poligon → Manifold → poligon **lulus**. Face hasil pengelompokan ulang planar dalam 1e-9 m, dan volumenya eksak untuk keperluan praktis. Kriteria keluar S3 terpenuhi.
2. Pada koordinat UTM absolut, residu planaritas 1,6e-7 m. Angka ini masih jauh di bawah 1 mm, tetapi 160 kali lebih besar dari toleransi nanometer. Kesimpulannya sama dengan S2: geometri harus disimpan relatif terhadap origin proyek, dan toleransi kernel tidak boleh dipakai pada koordinat absolut.
3. Untuk jalur A, `manifold-csg` punya target `wasm32-unknown-unknown`, tetapi berstatus **provisional**:
   - Butuh fitur `unstable-wasm-uu` dan LLVM 20+.
   - Tidak ada runtime exception, jadi exception di STL langsung membuat program abort.
   - I/O OBJ tidak tersedia.

   Belum diuji di spike ini. Ini relevan untuk kriteria #5 (Hari 11). Alternatifnya, Hew sudah punya boolean sendiri (`boolean.rs`).

Belum diuji: bukaan yang menembus dinding berketebalan (model LOD3 dengan dinding solid), dan sliver pada kasus hampir koplanar.
