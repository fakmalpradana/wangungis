# 2026-10-29 Hari 4 — spike S1-b: baca kernel push/pull Hew (kriteria #4)

Tujuan: pahami alur push/pull Hew, tulis penjelasan 1 halaman.

- Dibaca: `docs/dev/ARCHITECTURE.md`; jalur `wasm-api Scene::push_pull` → `Document::apply_object_op` → `History::apply` → `Object::push_pull` (`ops.rs:2651`, 7 langkah clone–ubah–validasi–tukar); `tol.rs`.
- Hasil: `docs/research/hew-kernel.md` + 5 pertanyaan uji pemahaman untuk manusia.
- Temuan: kernel WASM juga di desktop (konfirmasi review Gemini). Kernel 65 ribu baris, dependensi cuma slotmap/serde/zip/tracing. Predikat eksak hanya orientasi 2D.
- Temuan presisi: `POINT_MERGE = PLANE_DIST = 1e-9` m < ulp f64 di Y≈9,15e6 (1,86e-9 m). Hipotesis: operasi di koordinat absolut northing Indonesia bisa ditolak. Uji Hari 6 (absolut vs lokal+origin).
- Belum dibaca: `boolean.rs` (push-through), `push_pull_build_walls`, `validate_sweep_result`.
- Kriteria #4: menunggu manusia menjawab 5 pertanyaan.

Langkah berikut: manusia uji pemahaman; Hari 5 buka issue scope geospasial di Hew (kriteria #6, butuh persetujuan karena posting publik).

Tambahan (mode otonom s.d. 11.00, diizinkan manusia):
- Hari 6, spike S2 (`research/hew-spike-s2-utm.md`): hipotesis terbukti. Kotak diputar 30° di Y≈9,15e6 → 3–13/100 push/pull ditolak; debug panic di clone (aman di release). Lokal/10 km → 0 tolak. #3 lulus bersyarat (pakai origin lokal).
- Hari 5: draf issue Hew (`research/hew-issue-draft.md`), TIDAK diposting (butuh persetujuan manusia).
- Hari 8–9: CI `validate-lod` 3 OS + fixture LOD1 + `scripts/validate_lod.py` di PR #2.
- Hari 10, spike S3 (`research/spike-s3-manifold.md`): proptest 1.000 kasus lulus lokal; UTM absolut residu planaritas 1,6e-7 m.
- Hari 7, spike FPS (`research/hew-spike-fps.md`): Chromium headless + Metal M4. 16,6 ribu Object = 14 FPS (33 ribu draw/frame); 167 ribu Object = crash. 1.024 Object × 1.000 face = 122 FPS. #2 gagal untuk kota; batas = 2 draw call per Object.
- CI PR #2 hijau 3 OS (log dicek: PASS kedua fixture di tiap OS). Tidak di-merge sendiri: menunggu review manusia (AGENTS §12).
- Hari 11 pra-evaluasi #5 (`research/hew-spike-ext.md`): crate di luar pohon Hew, CRS/origin di attribute dictionary (round-trip bit-identik), ekspor CityJSON lolos val3dity 2/2. Celah: atribut per face, UI. #5 lulus sebagian.
