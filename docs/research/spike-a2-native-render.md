# Spike A2 — kernel Hew native + renderer wgpu ber-batching

Tanggal: 2026-10-29. Menguji premis opsi A2 (ADR-000): bila crate inti Hew dipakai secara native dan seluruh Object digabung ke satu buffer, apakah model kota besar bisa dimuat dan diorbit dengan lancar?

Setup:
- Crate `~/Desktop/3_dummy/wangungis-spikes/s6-native-render` memakai `kernel` + `tessellate` Hew v1.3.1 (path dependency, tanpa mengubah Hew), `wgpu` 30.0.1 (backend Metal, adapter "Apple M4"), `pollster`, dan `bytemuck`.
- Semua Object di-tessellate, lalu digabung menjadi **satu** buffer segitiga dan **satu** buffer garis edge. Jadi 2 draw call per frame, berapa pun jumlah Object.
- Render offscreen 1920×1080 dengan depth, tanpa MSAA, shading Lambert sederhana. Orbit 300 frame.
- Waktu per frame diukur dari submit sampai GPU selesai (`device.poll(wait)`).
- Sumber: [`spike-a2-native-render.rs`](spike-a2-native-render.rs).

## Hasil (model sama dengan spike FPS web)

| Model | Object | Face | Segitiga | Muat `.hew` | Tessellate + gabung | RSS | Frame median | FPS | Web Hew (pembanding) |
|---|---|---|---|---|---|---|---|---|---|
| Grid 129² | 16.641 | 99.846 | 199.692 | 0,12 s | 0,05 s | 0,2 GB | 0,93 ms | 1.075 | 14,4 FPS |
| Grid 289² | 83.521 | 501.126 | 1.002.252 | 0,52 s | 0,25 s | 0,9 GB | 2,41 ms | 415 | 2,2 FPS |
| Grid 409² | 167.281 | 1.003.686 | 2.007.372 | 1,06 s | 0,49 s | 1,8 GB | 4,30 ms | **233** | tab crash |

## Revisi: dokumen, buffer, dan indeks dimuat bersamaan, plus tes edit (2026-10-29, setelah review Gemini)
Versi pertama membebaskan `Document` sebelum mengukur render, dan indeks `inference` diukur di program terpisah. Versi ini menyimpan semuanya di memori sekaligus, lalu menguji satu edit.

| Model | Object / face | Muat + tessellate | Indeks + snap pertama | **Waktu sampai interaktif** | RSS total | FPS | Snap stabil | Edit 1 push/pull | Snap pertama **setelah edit** |
|---|---|---|---|---|---|---|---|---|---|
| Kota tiruan 25k × 40 face | 24.964 / 998.560 | 0,97 + 0,58 s | 6,25 s | **7,8 s** | 3,5 GB | **203** | 0,9 ms | 0,1 ms (+ indeks ulang dan tessellate ulang 1 Object 22 ms) | **5,9 s** |
| Grid kotak 409² | 167.281 / 1.003.686 | 1,03 + 0,49 s | 3,71 s | **5,2 s** | 2,7 GB | 232 | 31 µs | 0,03 ms (+ 17 ms) | **3,5 s** |

Pada model kota yang sama, web Hew mencapai 8,6 FPS dengan waktu muat 10 detik.

**Masalah baru, berlaku untuk jalur A dan A2:**
- `InferenceScene` di Hew membangun ulang seluruh indeks spasial secara lazy setelah *setiap* mutasi. Komentar di kodenya: "mark it dirty for a lazy full rebuild on the next query (per-committed-op, never per-frame)".
- Di skala kota, setiap edit diikuti jeda 3,5–6 detik pada gerakan kursor berikutnya. Waktu sampai interaktif (5–8 detik) juga melewati target ≤ 3 detik.
- Perbaikan: indeks inkremental, misalnya BVH per Object + BVH tingkat atas, atau indeks per tile. Ini perubahan desain di crate `inference`: butuh PR upstream atau fork. Estimasi **asumsi** 20–40 jam.

## Mitigasi tanpa mengubah Hew: satu `InferenceScene` per tile 100 m

Sumber: [`spike-a2-inference-tiled.rs`](spike-a2-inference-tiled.rs). Setiap Object dimasukkan ke scene milik tile tempat vertex pertamanya berada. Edit hanya membuat tile itu kotor, dan query hanya menyentuh tile di bawah kursor.

| Model | Tile (Object/tile) | Bangun | Pemanasan semua tile (serial) | Daftar ulang 1 Object | Snap pertama **setelah edit** | Berikutnya |
|---|---|---|---|---|---|---|
| Kota tiruan 25k × 40 | 380 (±65) | 0,30 s | 2,30 s | 0,1 ms | **7,8 ms** (sebelumnya 5,9 s) | 35 µs |
| Grid kotak 167k | 2.009 (±83) | 0,16 s | 0,56 s | 0,007 ms | **0,48 ms** (sebelumnya 3,5 s) | 8 µs |

Kesimpulan:
- Jeda setelah edit **hilang** dengan wrapper per tile di atas crate `inference` yang tidak diubah.
- Pemanasan per tile saling independen, jadi bisa diparalelkan antar-thread karena setiap tile punya scene sendiri. Dengan begitu waktu sampai interaktif untuk model kota kemungkinan turun ke ±2 detik. Angka ini **asumsi** dan belum diukur.
- Yang belum ditangani:
  - Snapping lintas batas tile: query tile tetangga, lalu gabungkan hasilnya dengan aturan prioritas `SnapKind` yang sama.
  - Garis inference panjang yang melewati banyak tile.
  - Object yang lebih besar dari satu tile.
- Estimasi **asumsi**: 10–20 jam. Lebih murah daripada memodifikasi `inference` (20–40 jam) dan tidak butuh persetujuan upstream.

## Kesimpulan
1. Masalah #2 di Hew sepenuhnya berasal dari lapisan web/renderer, bukan dari kernel.
2. Dengan kernel Hew yang dipakai native dan batching sederhana, model 1M face / 167 ribu Object berjalan **233 FPS**. Itu ±8× di atas target v1.0 untuk 10M face (≥ 30 FPS) bila diskalakan linear, dan ini masih **asumsi**.
3. Muat + tessellate butuh 1,55 detik. Tetapi **waktu sampai interaktif 5,2–7,8 detik** karena indeks snapping (lihat revisi di atas), jadi target ≤ 3 detik **belum** terpenuhi.
4. Premis opsi A2 terbukti untuk render. Untuk memori, 3,5 GB pada model 1M face masih di bawah RAM 16 GB, tetapi bukan angka kecil. Yang belum dibuktikan:
   - Picking per Object di buffer gabungan, misalnya lewat ID buffer atau BVH dari crate `inference` Hew.
   - Update inkremental saat satu Object diedit: buffer harus di-patch per rentang, bukan dibangun ulang semuanya.
   - MSAA dan edge bergaya CAD.
   - UI egui.
   - RTC: posisi dari `tessellate` berupa f32 koordinat world, jadi geometri harus sudah lokal (lihat S2).

## Tambahan: snapping dan picking dengan crate `inference` Hew (native)

Sumber: [`spike-a2-inference.rs`](spike-a2-inference.rs). Setup:
- Semua Object dimasukkan ke `InferenceScene`.
- Lalu dijalankan 1.000 query dari kamera setinggi ±120 m ke titik acak di seluruh scene.
- Jenis query: `resolve` (snap dengan cone aperture 0,01) dan `pick_face`.

| Model | Object | Bangun indeks | Query snap pertama | Snap median / p95 / maks (setelah query pertama) | `pick_face` median / p95 |
|---|---|---|---|---|---|
| Grid 129² | 16.641 | 14 ms | ±0,26 s | 12 / 14 µs | 1 / 2 µs |
| Grid 409² | 167.281 | 143 ms | **3,6 s** | 16 / 26 / 65 µs | 1 / 2 µs |

Kesimpulan: snapping dan picking Hew tetap interaktif pada 167 ribu Object. Yang perlu ditangani:
- Query pertama membangun struktur tambahan secara lazy dan butuh 3,6 detik. Solusinya: picu sekali di thread latar setelah model dimuat.
- `pick_face` mengenai 943 dari 1.000 sinar. Sisanya jatuh di celah antar-bangunan, jadi angka ini wajar.
