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
- #7 **lulus** (2026-10-29): manusia menerima AGPL-3.0-only (keputusan K1).

Keputusan: ≥ 6 dari 7 lulus, termasuk #3 dan #7 → A. Selain itu → B, dengan Hew sebagai referensi.

## Konsekuensi
- ADR-001..007 ditulis untuk kedua jalur; bagian yang bergantung jalur diberi tanda.
- Tidak menyalin kode Hew ke jalur B kecuali lisensi proyek AGPL.
- Kernel Hew dikompilasi ke WebAssembly (`wasm32-unknown-unknown`). Crate dengan dependensi C/C++ (`proj` dengan `bundled_proj`, `manifold-csg`) tidak bisa begitu saja masuk ke kernel WASM. Pada jalur A, crate ini kemungkinan harus berjalan di sisi native Tauri. Hal ini diuji sebagai bagian kriteria #5 (Hari 11). Ditemukan dalam review silang Gemini, 2026-10-29.
- Kriteria #2 (≥ 30 FPS) adalah ambang minimum spike. Target produk tetap ≥ 60 FPS pada 1M face di v0.2 (`plan/produk.md`), jadi selisihnya harus ditutup sebelum v0.2.
