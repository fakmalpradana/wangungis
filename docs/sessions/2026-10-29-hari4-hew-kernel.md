# 2026-10-29 Hari 4 — spike S1-b: baca kernel push/pull Hew (kriteria #4)

Tujuan: pahami alur push/pull Hew, tulis penjelasan 1 halaman.

- Dibaca: `docs/dev/ARCHITECTURE.md`; jalur `wasm-api Scene::push_pull` → `Document::apply_object_op` → `History::apply` → `Object::push_pull` (`ops.rs:2651`, 7 langkah clone–ubah–validasi–tukar); `tol.rs`.
- Hasil: `docs/research/hew-kernel.md` + 5 pertanyaan uji pemahaman untuk manusia.
- Temuan: kernel WASM juga di desktop (konfirmasi review Gemini). Kernel 65 ribu baris, dependensi cuma slotmap/serde/zip/tracing. Predikat eksak hanya orientasi 2D.
- Temuan presisi: `POINT_MERGE = PLANE_DIST = 1e-9` m < ulp f64 di Y≈9,15e6 (1,86e-9 m). Hipotesis: operasi di koordinat absolut northing Indonesia bisa ditolak. Uji Hari 6 (absolut vs lokal+origin).
- Belum dibaca: `boolean.rs` (push-through), `push_pull_build_walls`, `validate_sweep_result`.
- Kriteria #4: menunggu manusia menjawab 5 pertanyaan.

Langkah berikut: manusia uji pemahaman; Hari 5 buka issue scope geospasial di Hew (kriteria #6, butuh persetujuan karena posting publik).

Lanjutan (mode otonom): `2026-10-29-otonom-spike.md`.
