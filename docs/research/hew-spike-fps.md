# Spike FPS — viewport Hew pada ±1M face (kriteria #2)

Tanggal: 2026-10-29. Hew v1.3.1 (`612c1c1`), build production web dengan `VITE_HEW_TEST=1` (diserve lewat `vite preview`).

Cara ukur:
- Chromium Playwright 1.62.1, headless, dipaksa memakai GPU **ANGLE Metal Apple M4**. Opsi: `--use-angle=metal --disable-gpu-vsync --disable-frame-rate-limit`.
- Viewport 1920×1080, DPR 1.
- Orbit kamera lewat `__hew_test.setCamera` di setiap `requestAnimationFrame` selama 8–10 detik.
- Draw call dihitung dengan membungkus `drawElements`/`drawArrays`.
- Script: [`hew-spike-fps.mjs`](hew-spike-fps.mjs). Model dibuat dengan `hew-cli run` (grid kotak 8×6×9 m, atau grid silinder 998 segmen).

Catatan: browser bawaan aplikasi tidak dipakai. Pane yang tidak sedang dilihat dibatasi ±2 FPS, jadi angkanya tidak valid.

## Hasil

| Model | Object | Face | File | Muat (ms) | Heap JS | FPS orbit | Draw call/frame | Frame median / p95 (ms) |
|---|---|---|---|---|---|---|---|---|
| Grid kotak 129² | 16.641 | 99.846 | 11 MB | 552 | 521 MB | **14,4** | 33.289 | 69 / 84 |
| Grid kotak 204² | 41.616 | 249.696 | 29 MB | 1.434 | 1,1 GB | **5,6** | 83.239 | 182 / 199 |
| Grid kotak 289² (dev build) | 83.521 | 501.126 | 57 MB | 3.315 | 2,5 GB | **2,2** | — | 396 / 551 |
| Grid kotak 409² (dev build) | 167.281 | 1.003.686 | 115 MB | — | — | **tab crash** saat dimuat | — | — |
| Grid silinder 32² × 1.000 face | 1.024 | ±1.024.000 | 150 MB | 17.756 | 624 MB | **122** | 2.055 | 8,4 / 13,3 |

Build dev dan production memberi FPS yang sama (14,4 dan 5,6), jadi biayanya bukan di JavaScript yang tidak diminifikasi.

## Kesimpulan

1. **GPU tidak menjadi batas.** 1M face dalam 1.024 Object mencapai 122 FPS.
2. **Jumlah Object yang menjadi batas.** Setiap Object = 2 draw call per frame (face + edge), tanpa batching. Waktu frame naik linear dengan jumlah Object.
   - Pada 167 ribu Object, tab crash saat memuat. Dugaan: batas memori WASM 32-bit (4 GB) atau memori renderer. hew-cli native untuk model yang sama memakai RSS 2,4 GB.
3. **Untuk skenario WangunGIS (kota LOD2):**
   - Satu bangunan = satu Object dengan ±20–60 face. Artinya 1M face ≈ 17–50 ribu Object.
   - Ekstrapolasi dari tabel: ±5–14 FPS. **Kriteria #2 (≥ 30 FPS) gagal untuk skenario kota**, dan lulus hanya untuk model dengan sedikit Object besar.
4. **Bisa diperbaiki**, tetapi pekerjaannya di renderer three.js Hew:
   - Batching Object statis (mis. `BatchedMesh` atau buffer gabungan per tile), dengan picking per-batch.
   - Opsi lain: LOD rendering (bangunan jauh memakai LOD1).
   - Ini bukan perubahan kecil, dan bisa berbenturan dengan arsitektur "satu Object = satu mesh" di Hew.
   - Estimasi kasar (**asumsi**): 40–80 jam, plus persetujuan pengelola bila ingin masuk upstream.
5. Memuat model 150 MB butuh 17,8 detik, sedangkan target produk ≤ 3 detik untuk 1M face. Ini juga perlu ditangani.

Yang belum diukur: aplikasi desktop Tauri (WKWebView, bukan Chromium), resolusi Retina (DPR 2), dan model LOD2 nyata (atap miring, 3D BAG).
