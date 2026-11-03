# SPEC — `ui` (binary `wangungis`)

## Tujuan
Shell aplikasi desktop F0. Membuka `.hew` atau proyek `.wangun`, menampilkannya dengan renderer `render`, dan menyediakan navigasi orbit/pan/zoom (#31) serta zoom extents (#32).

## Perilaku
- `wangungis [file.hew | file.wangun]`: membuka file dari argumen. Bisa juga lewat kotak path + tombol **Buka** (atau Enter).
- File dibuka lewat `scene::project::Project::open`. Setiap tile kernel menjadi satu tile render di pusatnya.
- `.hew` yang geometrinya melewati 1,5 km langsung dipecah menjadi tile 1 km di memori. Status bar menandainya "belum disimpan", dan tombol **Simpan proyek (.wangun)** menulis `<nama>.wangun` di sebelah file asli. File asli tidak pernah ditimpa.
- Mouse:
  - Seret kiri: orbit.
  - Seret tengah/kanan atau Shift+seret kiri: pan.
  - Scroll: zoom.
- **Zoom extents**: seluruh model masuk ke layar (bola pembatas di dalam kerucut pandang).
- Status bar:
  - Nama file, jumlah tile, jumlah objek, dan jumlah segitiga.
  - EPSG + origin proyek, atau "Tanpa origin proyek".
  - Waktu per frame.
- Error baca/parse ditampilkan di status bar dan tidak membuat aplikasi crash. Contohnya: origin rusak, wadah tidak valid, atau objek lebih panjang dari ±1 km (`TooFar`).

## Struktur
- `camera.rs`: `Orbit`, logika kamera murni `f64`, dengan test unit tanpa GPU (jarak mata, batas pitch, pan tegak lurus arah pandang, batas zoom, fit).
- `main.rs`: penggabungan bounds lintas tile (`union_bounds`, ada test unit) dan app eframe 0.37 (wgpu). Viewport 3D dirender ke texture offscreen milik kita, lalu ditampilkan sebagai gambar egui. Dengan begitu depth buffer dikelola `render`, bukan egui.

## Non-goals F0 (menyusul)
- Dialog file native.
- Form pengisian origin.
- Lazy-load per tile.
- Seleksi/picking.
- Tool gambar (v0.1).
