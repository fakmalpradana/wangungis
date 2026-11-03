# SPEC — `ui` (binary `wangungis`)

## Tujuan
Shell aplikasi desktop F0. Membuka satu `.hew`, menampilkannya dengan renderer `render`, dan menyediakan navigasi orbit/pan/zoom (#31) serta zoom extents (#32).

## Perilaku
- `wangungis [file.hew]`: membuka file dari argumen. Bisa juga lewat kotak path + tombol **Buka** (atau Enter).
- Mouse:
  - Seret kiri: orbit.
  - Seret tengah/kanan atau Shift+seret kiri: pan.
  - Scroll: zoom.
- **Zoom extents**: seluruh model masuk ke layar (bola pembatas di dalam kerucut pandang).
- Status bar:
  - Nama file, jumlah objek, dan jumlah segitiga.
  - EPSG + origin proyek, atau "Tanpa origin proyek".
  - Peringatan bila geometri > 1,5 km dari origin dokumen (`KERNEL_FRAME_BOUND_M`), karena edit kernel bisa ditolak.
  - Waktu per frame.
- Error baca/parse ditampilkan di status bar dan tidak membuat aplikasi crash.

## Struktur
- `camera.rs`: `Orbit`, logika kamera murni `f64`, dengan test unit tanpa GPU (jarak mata, batas pitch, pan tegak lurus arah pandang, batas zoom, fit).
- `main.rs`: app eframe 0.37 (wgpu). Viewport 3D dirender ke texture offscreen milik kita, lalu ditampilkan sebagai gambar egui. Dengan begitu depth buffer dikelola `render`, bukan egui.

## Non-goals F0 (menyusul)
- Dialog file native.
- Form pengisian origin.
- Proyek multi-tile.
- Seleksi/picking.
- Tool gambar (v0.1).
