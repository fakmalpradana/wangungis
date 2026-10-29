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

## Kesimpulan
1. Masalah #2 di Hew sepenuhnya berasal dari lapisan web/renderer, bukan dari kernel.
2. Dengan kernel Hew yang dipakai native dan batching sederhana, model 1M face / 167 ribu Object berjalan **233 FPS**. Itu ±8× di atas target v1.0 untuk 10M face (≥ 30 FPS) bila diskalakan linear, dan ini masih **asumsi**.
3. Waktu buka 1,55 detik total, memenuhi target ≤ 3 detik.
4. Premis opsi A2 terbukti untuk render dan memori. Yang belum dibuktikan:
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
