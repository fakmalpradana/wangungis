# ADR-000: Strategi terhadap Hew — spike evaluasi dulu

- Status: **Accepted** (dipilih manusia 2026-10-28: jalur C). Spike ditutup 2026-11-01 dengan keputusan K2 = **A2** (lihat bagian terakhir).
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
- #4 **tidak dijalankan** (lihat Keputusan K2). Penjelasan alur push/pull dan 5 pertanyaan uji ada di `docs/research/hew-kernel.md`. Lulus bila manusia bisa menjawab tanpa melihat dokumen.
- #2 **gagal untuk skenario kota** (2026-10-29, `docs/research/hew-spike-fps.md`).
  - GPU sanggup: 1M face dalam 1.024 Object = 122 FPS.
  - Tetapi setiap Object = 2 draw call tanpa batching: 16,6 ribu Object = 14 FPS, 41,6 ribu = 5,6 FPS, dan 167 ribu Object membuat tab crash saat dimuat.
  - Diukur langsung pada kota tiruan (25 ribu Object × 40 face = 1M face): **8,6 FPS**, waktu muat 10 detik.
  - Bisa diperbaiki dengan batching renderer (estimasi **asumsi** 40–80 jam).
  - Diukur di Chromium headless + Metal, bukan di Tauri WKWebView dengan DPR 2. Jadi angka ini proksi.
- #5 **lulus sebagian, sisanya bergantung pada #6** (pra-evaluasi 2026-10-29, `docs/research/hew-spike-ext.md`).
  - Crate di luar pohon Hew bisa menyimpan CRS + origin di attribute dictionary (round-trip bit-identik) dan mengekspor CityJSON yang lolos val3dity, tanpa mengubah kernel.
  - Atribut Object bertahan melewati push/pull, undo, redo, dan reload (diuji dengan assert).
  - Celah: belum ada atribut per face, dan integrasi UI butuh perubahan `wasm-api`/`app`, yaitu PR upstream atau fork.
  - Indeks snapping (`inference`) dibangun ulang penuh setelah setiap edit, sehingga muncul jeda 3,5–6 detik di skala kota. Perbaikannya juga butuh PR atau fork.
- #3 **gagal pada koordinat absolut** (2026-10-29, `docs/research/hew-spike-s2-utm.md`). Kriteria ini lulus hanya dengan koordinat lokal + origin proyek, dan Hew belum punya mekanisme itu.
  - Render: viewport tanpa RTC, posisi `f32` world. Kotak sedalam 4,244 m di UTM tergambar ≈ 5,00 m (galat 0,76 m).
  - Kernel: lingkaran r = 3 m di UTM absolut ditolak saat diekstrusi.
  - Sweep 36 sudut: 0 penolakan sampai ±2,1e6 m per sumbu. Usulan batas koordinat lokal kernel: ±1.000 km dari origin proyek.
  - Push/pull yang diterima selalu presisi (galat ≤ 1,5e-9 m, metrik pencocokan vertex satu-satu).
  - Kotak yang diputar 30° di koordinat UTM absolut (Y ≈ 9,15e6) membuat 3–13% push/pull ditolak (`NonManifoldResult`). Penyebabnya toleransi absolut 1e-9 m yang lebih kecil dari ulp `f64`.
  - Dengan koordinat lokal, termasuk pada jarak 10 km: 0 penolakan.
  - Syarat lulus: geometri disimpan relatif terhadap origin proyek (ADR-006).
  - UI Hew belum punya lapisan origin-shift.
- #7 **lulus** (2026-10-29): manusia menerima AGPL-3.0-only (keputusan K1).

Status sementara (2026-10-29, setelah review silang Gemini):
- Lulus: #1, #7.
- Gagal: #2.
- Gagal pada koordinat absolut: #3 (lulus hanya dengan origin lokal, dan mekanisme ini belum ada di Hew).
- Bergantung pada #6: #5.
- Belum diuji: #4, #6.

Menurut aturan di bawah, jalur A sudah **tidak bisa** lulus: #2 gagal, sehingga semua kriteria lain wajib lulus, padahal #5 tidak bisa lulus penuh tanpa #6. Biaya yang harus masuk ke estimasi jalur A bila tetap dipilih:
- Batching renderer: 40–80 jam.
- Indeks inference inkremental: 20–40 jam.
- Lapisan origin-shift di UI.

Keputusan: ≥ 6 dari 7 lulus, termasuk #3 dan #7 → A. Selain itu → B, dengan Hew sebagai referensi.

## Konsekuensi
- ADR-001..007 ditulis untuk kedua jalur; bagian yang bergantung jalur diberi tanda.
- Tidak menyalin kode Hew ke jalur B kecuali lisensi proyek AGPL.
- Kernel Hew dikompilasi ke WebAssembly (`wasm32-unknown-unknown`). Crate dengan dependensi C/C++ (`proj` dengan `bundled_proj`, `manifold-csg`) tidak bisa begitu saja masuk ke kernel WASM. Pada jalur A, crate ini kemungkinan harus berjalan di sisi native Tauri. Hal ini diuji sebagai bagian kriteria #5 (Hari 11). Ditemukan dalam review silang Gemini, 2026-10-29.
- Kriteria #2 (≥ 30 FPS) adalah ambang minimum spike. Target produk tetap ≥ 60 FPS pada 1M face di v0.2 (`plan/produk.md`), jadi selisihnya harus ditutup sebelum v0.2.

## Usulan opsi A2 dari hasil spike (2026-10-29; riwayat, sudah diputuskan di K2)

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
  - v0.1 ±100–130 jam (asumsi awal). Ada dua koreksi dari review Gemini:
    - (a) Orkestrasi di `crates/wasm-api` (±24,7 ribu baris, mis. routing push-through ke boolean) juga harus diport atau dipakai ulang.
    - (b) Indeks inference inkremental (20–40 jam) dibutuhkan baik di A maupun A2.

    Rentang yang lebih jujur: **v0.1 ±140–200 jam**. Pembanding: jalur A dengan biaya perbaikan menjadi ±100–160 jam, dan jalur B ±200 jam.

**Bukti awal (2026-10-29, `docs/research/spike-a2-native-render.md`):** model 1M face / 167 ribu Object yang membuat web Hew crash berhasil dimuat native dalam 1,06 detik. Model itu dirender dengan wgpu ber-batching pada **233 FPS** (1920×1080, 2 draw call per frame, RSS 1,8 GB). Revisi dengan dokumen, buffer, dan indeks dimuat bersamaan: model kota 25 ribu × 40 face mencapai 203 FPS (web Hew 8,6 FPS), RSS 3,5 GB, dan waktu sampai interaktif 7,8 detik. Snap pertama setelah edit butuh 5,9 detik, masalah yang sama dengan jalur A. Mitigasi tanpa mengubah Hew (satu `InferenceScene` per tile 100 m) menurunkannya ke **7,8 ms**; snapping lintas tile belum ditangani (estimasi 10–20 jam, asumsi). UI belum diuji.

**Usulan aturan keputusan tiga jalur** (riwayat; tidak pernah disetujui, lihat Keputusan K2):
- **A**: hanya bila #2 lulus setelah perbaikan yang diterima upstream, dan #4, #5, #6 lulus.
- **A2**: bila #1, #3 (bagian kernel), dan #4 lulus, dan dependensi git ke crate inti Hew bisa dipin dan disinkronkan (kriteria ini belum diukur). Persetujuan pengelola (#6) hanya diperlukan untuk perubahan kernel seperti atribut per face dan indeks inkremental. Tanpa persetujuan itu, kita memelihara patch kecil di fork.
- **B**: bila #4 gagal (kernel tidak bisa dipahami), atau bila sinkronisasi dengan Hew terbukti terlalu mahal.

Ini usulan, bukan keputusan. Keputusan akhirnya ada di bagian berikut.

## Keputusan K2 (2026-11-01): jalur A2

Manusia mendelegasikan keputusan kepada agen: "keputusan saya mengikuti rekomendasi terbaik anda, jadi lanjutkan" (2026-11-01). Agen memilih **A2**, sesuai rekomendasi di `research/keputusan-k2.md`.

**K2 mengesampingkan kedua aturan keputusan di ADR ini, bukan mengikutinya:**
- Aturan biner asli (≥ 6/7 lulus → A, selain itu → B) menghasilkan **B**, karena hanya #1 dan #7 yang lulus.
- Usulan aturan tiga jalur mensyaratkan #4 lulus untuk A2. #4 tidak dijalankan, dan `keputusan-k2.md` sendiri menyebut A2 kehilangan alasannya bila #4 gagal.

Alasan tetap memilih A2:
- Bukti terukur di sisi kernel dan render: 203 FPS lawan 8,6 FPS, kernel lulus pada koordinat lokal, dan ekspor CityJSON lolos val3dity.
- Pemilik mendelegasikan keputusan secara eksplisit.
- Jalur B tidak mengatasi masalah #4. Bila pemilik tidak menilai kode kernel, kernel buatan sendiri juga tidak akan dinilainya, padahal biayanya lebih besar.

Penyimpangan, dicatat apa adanya:
- **#4 tidak diuji.** Manusia menyatakan belum bisa menilai kode kernel. Syarat "#4 lulus" untuk A2 tidak terpenuhi. Penggantinya (lihat `plan/risiko.md` R6):
  - Review silang agen lain (Gemini) untuk kebenaran pada setiap PR.
  - Test otomatis wajib.
  - Uji perilaku oleh 3D engineer pada rilis beta. Umpan baliknya menjadi sumber utama perbaikan.
- **#6 ditunda.** A2 bisa mulai tanpa persetujuan pengelola Hew. Ada tiga celah yang kelak butuh perubahan upstream atau patch fork:
  - Atribut per face (semantik permukaan, ADR-006).
  - Metrik ranking snap ("gravity") sebagai API publik. Tanpa ini, snapping per tile hanya 98% identik dengan scene global (`research/spike-a2-native-render.md`).
  - Indeks inference inkremental. Draf issue tetap di `research/hew-issue-draft.md`. Draf itu diposting baru ketika kita butuh perubahan kernel (misalnya atribut per face), dan selalu dengan persetujuan eksplisit manusia karena tampil publik atas nama akunnya.
- **Sinkronisasi crate Hew belum diukur.** Ini diukur pada pembaruan pin pertama. Bila terlalu mahal, pertimbangkan pindah ke B (aturan di atas).

Konsekuensi:
- ADR-001..005 dan ADR-007 menjadi Accepted untuk A2.
- ADR-006 Accepted dengan format `.hew` + atribut namespace `wangungis.*`.
- Celah yang sudah terukur dan wajib ditangani di F0/v0.1:
  - Renderer wgpu ber-batching + RTC (ADR-003).
  - Indeks inference per tile (prototipe 7,8 ms; snapping lintas tile belum).
  - Origin proyek + tile kernel 1 km: geometri ≤ 1,5 km dari frame `Document` (ADR-004; awalnya ±50 km, dikoreksi setelah property test).
