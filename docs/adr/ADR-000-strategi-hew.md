# ADR-000: Strategi terhadap Hew — spike evaluasi dulu

- Status: **Accepted** (dipilih manusia 2026-10-28: jalur C)
- Konteks: [Hew](https://github.com/hew3d/hew) sudah menyediakan UX gaya SketchUp di atas kernel half-edge Rust (AGPL-3.0-only), tetapi tidak punya fitur geospasial/LOD. Rincian di `docs/research/teknologi.md` §0.

## Keputusan
Jalankan spike evaluasi 1–2 minggu sebelum memilih antara:
- **A** — bangun di atas Hew (fork atau kontribusi upstream; lisensi AGPL-3.0-only), atau
- **B** — bangun dari nol (Hew hanya referensi UX; lisensi GPL-3.0-or-later).

## Kriteria spike (semua diukur, bukan dinilai)
| # | Kriteria | Lulus untuk A bila |
|---|---|---|
| 1 | Build lokal di M4 + test suite | build dan semua test hijau < 1 jam kerja |
| 2 | FPS viewport pada 1M face (desktop Tauri) | ≥ 30 FPS orbit |
| 3 | Koordinat UTM (mis. X=430000, Y=9150000 m) | tidak ada jitter terlihat; operasi push/pull tetap presisi ≤ 1 mm |
| 4 | Keterbacaan kernel | manusia bisa menjelaskan alur push/pull setelah 1 hari membaca |
| 5 | Titik ekstensi | crate LOD/geospasial bisa ditambahkan tanpa mengubah kernel inti, atau pengelola bersedia menerima PR |
| 6 | Respons pengelola | ada balasan atas issue/diskusi tentang scope geospasial dalam 2 minggu |
| 7 | Lisensi | manusia menerima AGPL-3.0-only untuk proyek |

Hasil sementara:
- #1 **lulus** (2026-10-29). Hew v1.3.1 (`612c1c1`) di M4 16 GB, clone bersih. Pemasangan toolchain + `cargo build --workspace` + `scripts/verify.sh` selesai hijau dalam 819 detik (±14 menit) tanpa intervensi.
  - Test Rust: 2.935 lulus, 0 gagal, 3 ignored.
  - Test app (Vitest): 6.219 lulus di 282 file.
  - Clippy `-D warnings` bersih.
  - Catatan untuk kriteria #4: kode Rust di `crates/` berjumlah ±279 ribu baris.
- #4 **menunggu manusia**. Penjelasan alur push/pull dan 5 pertanyaan uji ada di `docs/research/hew-kernel.md`. Lulus bila manusia bisa menjawab tanpa melihat dokumen.
- #2 **gagal untuk skenario kota** (2026-10-29, `docs/research/hew-spike-fps.md`).
  - GPU sanggup: 1M face dalam 1.024 Object = 122 FPS.
  - Tetapi setiap Object = 2 draw call tanpa batching: 16,6 ribu Object = 14 FPS, 41,6 ribu = 5,6 FPS, dan 167 ribu Object membuat tab crash saat dimuat.
  - Kota LOD2 dengan 1M face ≈ 17–50 ribu Object, perkiraan 5–14 FPS.
  - Bisa diperbaiki dengan batching renderer (estimasi **asumsi** 40–80 jam).
- #5 **lulus sebagian** (pra-evaluasi 2026-10-29, `docs/research/hew-spike-ext.md`).
  - Crate di luar pohon Hew bisa menyimpan CRS + origin di attribute dictionary (round-trip bit-identik) dan mengekspor CityJSON yang lolos val3dity, tanpa mengubah kernel.
  - Celah: belum ada atribut per face, dan integrasi UI butuh perubahan `wasm-api`/`app`, yaitu PR upstream atau fork.
- #3 **lulus bersyarat, bagian kernel** (2026-10-29, `docs/research/hew-spike-s2-utm.md`).
  - Push/pull yang diterima selalu presisi (galat ≤ 1,3e-9 m).
  - Kotak yang diputar 30° di koordinat UTM absolut (Y ≈ 9,15e6) membuat 3–13% push/pull ditolak (`NonManifoldResult`). Penyebabnya toleransi absolut 1e-9 m yang lebih kecil dari ulp `f64`.
  - Dengan koordinat lokal, termasuk pada jarak 10 km: 0 penolakan.
  - Syarat lulus: geometri disimpan relatif terhadap origin proyek (ADR-006).
  - Bagian render (jitter di viewport) belum diuji.
- #7 **lulus** (2026-10-29): manusia menerima AGPL-3.0-only (keputusan K1).

Status sementara (2026-10-29): #1, #7 lulus; #3 lulus bersyarat; #5 lulus sebagian; #2 gagal untuk skenario kota; #4 dan #6 belum. Akibatnya jalur A hanya mungkin bila #4, #5, dan #6 semuanya lulus, dan #3 bersyarat diterima. Biaya batching renderer harus masuk estimasi jalur A.

Keputusan: ≥ 6 dari 7 lulus, termasuk #3 dan #7 → A. Selain itu → B, dengan Hew sebagai referensi.

## Konsekuensi
- ADR-001..007 ditulis untuk kedua jalur; bagian yang bergantung jalur diberi tanda.
- Tidak menyalin kode Hew ke jalur B kecuali lisensi proyek AGPL.
- Kernel Hew dikompilasi ke WebAssembly (`wasm32-unknown-unknown`). Crate dengan dependensi C/C++ (`proj` dengan `bundled_proj`, `manifold-csg`) tidak bisa begitu saja masuk ke kernel WASM. Pada jalur A, crate ini kemungkinan harus berjalan di sisi native Tauri. Hal ini diuji sebagai bagian kriteria #5 (Hari 11). Ditemukan dalam review silang Gemini, 2026-10-29.
- Kriteria #2 (≥ 30 FPS) adalah ambang minimum spike. Target produk tetap ≥ 60 FPS pada 1M face di v0.2 (`plan/produk.md`), jadi selisihnya harus ditutup sebelum v0.2.
