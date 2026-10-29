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

## Usulan opsi A2 dari hasil spike (2026-10-29, menunggu keputusan manusia K2)

Spike memperlihatkan hal-hal berikut:
- Bagian Hew yang paling bernilai bagi kita adalah crate Rust murninya, dan semuanya tanpa dependensi UI:
  - `kernel` (65 ribu baris; dependensi hanya slotmap/serde/zip/tracing).
  - `inference` (8,9 ribu baris).
  - `tessellate` (4 ribu baris).
  - `mesh-export` (2,9 ribu baris).
- Dua kelemahan Hew untuk kita sama-sama berada di lapisan web:
  - Renderer three.js tanpa batching (#2).
  - Kernel yang terkunci di WASM: batas memori 4 GB, dan PROJ/Manifold tidak bisa dimuat.
- Prototipe #5 sudah memakai `kernel` secara **native** dari crate di luar pohon Hew, tanpa mengubah Hew.

**Opsi A2 — pakai crate inti Hew sebagai library, bangun aplikasi native sendiri:**
- **Reuse:** `kernel`, `inference`, `tessellate`, `mesh-export` sebagai dependensi git yang dipin ke satu commit.
- **Tulis sendiri:** shell aplikasi + tool interaksi dengan egui, dan renderer wgpu dengan batching dan RTC. Tool UI Hew (±37 ribu baris TypeScript) dipakai sebagai referensi perilaku. Porting langsung dengan atribusi juga boleh karena AGPL sudah diterima (K1).
- **Lisensi:** AGPL-3.0-only.

Perbandingan dengan jalur A, untuk kebutuhan kita:
- **Kelebihan:**
  - Tidak ada batas WASM 4 GB.
  - PROJ, Manifold, dan GDAL bisa dipakai langsung.
  - Renderer dirancang untuk banyak bangunan sejak awal.
  - Tidak bergantung pada persetujuan pengelola untuk UI. Kriteria #6 hanya menyangkut perubahan kernel seperti atribut per face.
- **Kekurangan:**
  - Seluruh UI/tool ditulis ulang.
  - API kernel Hew belum stabil (5 rilis dalam 11 hari), jadi butuh sinkronisasi berkala.
- **Estimasi (asumsi, belum diukur):**
  - F0 ±40–60 jam: renderer + shell.
  - v0.1 ±100–130 jam, di antara jalur A (40 jam) dan B (200 jam), karena kernel, push/pull, undo, dan inference tidak ditulis ulang.

**Bukti awal (2026-10-29, `docs/research/spike-a2-native-render.md`):** model 1M face / 167 ribu Object yang membuat web Hew crash berhasil dimuat native dalam 1,06 detik. Model itu dirender dengan wgpu ber-batching pada **233 FPS** (1920×1080, 2 draw call per frame, RSS 1,8 GB). Picking, edit inkremental, dan UI belum diuji.

Ini usulan, bukan keputusan. Keputusan A, A2, atau B tetap diambil manusia di Hari 12 (K2), setelah #4 dan #6 selesai.
