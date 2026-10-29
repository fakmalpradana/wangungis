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
- Spike A2 (`research/spike-a2-native-render.md`): kernel+tessellate Hew native + wgpu batching. 1M face/167 ribu Object = 233 FPS, muat 1,06 dtk, RSS 1,8 GB (web Hew crash). Premis A2 terbukti untuk render/memori.
- Spike A2 inference: 167 ribu Object, indeks 143 ms, snap 16/26 µs (median/p95), pick 1 µs; query pertama 3,6 dtk (struktur lazy).
- Review silang Gemini PR #3: 12 temuan, mayoritas valid. Diperbaiki dan diukur ulang:
  - S3: wajib 13 face, toleransi sudut sejati, boolean di frame akhir. Lokal tetap lulus; UTM hanya 1/200 utuh.
  - #5: atribut diuji melewati push/pull, undo, redo, reload.
  - S2: metrik pencocokan vertex satu-satu, kesimpulan sama.
  - Kota tiruan 25 ribu × 40 face diukur langsung: web Hew 8,6 FPS vs A2 203 FPS.
  - A2 dengan dokumen, buffer, dan indeks dimuat bersamaan: RSS 3,5 GB, waktu sampai interaktif 7,8 dtk.
  - Temuan baru: indeks inference dibangun ulang penuh tiap edit → jeda 3,5–6 dtk.
  - #3 diturunkan ke "belum lulus". Estimasi A2 direvisi ke 140–200 jam. Usulan aturan tiga jalur A/A2/B.
- #3 bagian render: viewport Hew tanpa RTC (f32 world). Kotak 4,244 m di UTM tergambar ±5,00 m (diukur dari piksel). Lingkaran r=3 m di UTM ditolak saat ekstrusi. #3 = gagal di koordinat absolut, lulus hanya dengan origin lokal.
- Mitigasi stall: InferenceScene per tile 100 m (tanpa ubah Hew). Snap setelah edit 5,9 dtk → 7,8 ms (kota), 3,5 dtk → 0,48 ms (grid). Lintas tile belum.
