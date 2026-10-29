# Tahap 1 — Definisi Produk

Status: draf 2026-10-28. Nama produk: WangunGIS (AGENTS.md §1). Dokumen ini masih memakai nama kerja lama "citySketch".

## Visi

Modeler 3D desktop open source tempat siapa pun yang pernah memakai SketchUp bisa langsung menggambar bangunan. Hasilnya langsung menjadi model kota LOD1–LOD3 yang tergeoreferensi, bersemantik, dan valid, tanpa plugin berbayar dan tanpa membersihkan geometri setelah ekspor.

## USP

1. **UX gambar-tangan yang familiar**: tool berbasis klik, inference, dan VCB seperti SketchUp.
2. **Geospasial sebagai fondasi, bukan plugin**: CRS dan origin `f64` per proyek. Presisi koordinat UTM diuji otomatis.
3. **Valid sejak digambar**: watertight, planaritas, orientasi normal, dan semantik dicek saat mengedit. Ekspor lolos val3dity.
4. **Cepat untuk banyak bangunan**: LOD1 batch dari footprint, template atap LOD2, template fasad LOD3, dan roof fitting dari DSM/point cloud.
5. **Gratis, open source, berjalan di macOS, Windows, dan Linux.**
6. **Pelengkap pipeline otomatis, bukan pesaingnya.** Pipeline seperti 3dfier dan roofer sudah menyelesaikan mayoritas bangunan secara batch. citySketch menjadi editor untuk mengoreksi sisanya dan menambah detail LOD3 ([riset](../research/kompetitor.md#3-pain-point-pengguna-0b)).

## Non-goals (sampai v1.0)

- Permukaan NURBS/organik dan modeling mekanik parametrik (wilayah Rhino, Plasticity, FreeCAD).
- Render fotorealistik offline. Cukup ekspor glTF/USD ke Blender.
- BIM penuh (struktur, MEP, penjadwalan). Cukup IFC dasar untuk pertukaran.
- Kolaborasi multi-user real-time dan layanan cloud.
- Kompatibilitas Ruby API dan ekstensi SketchUp (ADR-007).
- Pembacaan/penulisan `.skp` sebagai fitur yang dijanjikan (risiko IP; lihat teknologi.md §0).
- Aplikasi mobile/tablet.

## Persona

| Persona | Konteks | Kebutuhan utama | Frustrasi saat ini |
|---|---|---|---|
| **Rani — teknisi GIS pemda** | Memperbarui model 3D kota dari footprint dan LiDAR | Ratusan bangunan LOD1/LOD2 per minggu, CityGML valid, CRS lokal (UTM 48S/49S) | Pipeline otomatis gagal pada bangunan kompleks; edit manual di SketchUp merusak georeferensi dan semantik |
| **Bima — arsitek / perencana kota** | Konteks tapak untuk desain dan studi bayangan | Model massa cepat, bayangan tergeolokasi, impor konteks kota | Konteks kota di SketchUp butuh langganan; koordinat besar membuat model "bergetar" |
| **Sari — mahasiswa geodesi/arsitektur** | Tugas dan riset model kota | Gratis, mudah dipelajari, ekspor CityJSON untuk analisis | Tool profesional mahal; Blender sulit untuk pemula CAD |
| **Dimas — peneliti/pengembang digital twin** | Otomasi dan eksperimen ML | API Python, batch headless, format terbuka | Tool GUI tidak bisa di-script; tool skrip tidak punya editor interaktif untuk koreksi |

## User stories (inti)

1. Sebagai Rani, saya mengimpor shapefile footprint beserta atribut tinggi, lalu menghasilkan ratusan bangunan LOD1 dalam satu perintah.
2. Sebagai Rani, saya memilih bangunan dan menerapkan template atap hip, lalu menyesuaikan kemiringan lewat VCB.
3. Sebagai Rani, saya menjalankan roof fitting dari DSM, lalu memperbaiki hasil yang salah dengan tool push/pull biasa.
4. Sebagai Rani, saya mengekspor CityGML 2.0/3.0 dan CityJSON yang lolos val3dity tanpa perbaikan manual.
5. Sebagai Bima, saya menggambar massa bangunan dengan Line, Rectangle, Push/Pull, dan inference seperti di SketchUp.
6. Sebagai Bima, saya menyalakan bayangan pada tanggal dan jam tertentu di lokasi proyek.
7. Sebagai Bima, saya bekerja di koordinat UTM asli tanpa jitter dan tanpa memindahkan model ke origin.
8. Sebagai Sari, saya membuka contoh 3D BAG CityJSON, mengedit satu bangunan, dan menyimpannya kembali tanpa kehilangan semantik.
9. Sebagai Sari, saya memakai UI berbahasa Indonesia.
10. Sebagai Dimas, saya menjalankan skrip Python yang memuat proyek, menerapkan template fasad ke semua bangunan, lalu mengekspor 3D Tiles.
11. Sebagai Dimas, saya menyimpan satu bangunan dengan LOD1, LOD2, dan LOD3 sekaligus, lalu mengekspor LOD tertentu.
12. Sebagai semua persona, saya membatalkan (undo) operasi apa pun tanpa merusak model.
13. Sebagai Rani, saya membuka hasil pipeline otomatis (mis. 3D BAG/roofer) dalam CityJSON. Saya melompat ke bangunan yang gagal val3dity, memperbaikinya dengan tool biasa, lalu menyimpan tanpa kehilangan semantik dan atribut.

## MoSCoW per rilis

Rilis mengikuti Tahap 5. Pada jalur A (Hew), sebagian besar butir v0.1 sudah tersedia dan cukup diverifikasi.

| Rilis | Must | Should | Could | Won't (di rilis ini) |
|---|---|---|---|---|
| Fase 0 | CI 3 OS, spike Hew, spike presisi UTM, spike Manifold | val3dity di CI | — | UI final |
| v0.1 | Line, Rect, Circle, Arc, Push/Pull, Move/Rotate/Scale, Eraser, inference dasar, VCB, undo/redo, save/load, orbit/pan/zoom, Groups | Offset, Tape Measure | Follow Me | LOD tools |
| v0.2 | Proyek ber-CRS, origin `f64`, impor GeoJSON/Shapefile, LOD1 batch, ekspor CityJSON | Impor GeoTIFF terrain, reprojection | Impor OSM | Atap |
| v0.3 | Template atap (flat, shed, gable, hip), semantik permukaan, validasi + val3dity, ekspor CityGML 2.0 | Atap pada footprint L/T/U, mansard, overhang, roof fitting dari DSM, ekspor 3D Tiles | CityGML 3.0 | LOD3 |
| v0.4 | Komponen jendela/pintu, template fasad, multi-LOD per bangunan | Balkon, kolom, library komponen, konversi LOD3→LOD2→LOD1 | Ornamen, impor point cloud | Plugin |
| v0.5+ | — | Scenes, Section Plane, Shadows, API Python, UI Bahasa Indonesia | Styles, Dokumen 2D (setara LayOut), IFC dasar, AI-assist (segmentasi ONNX lokal) | Kolaborasi |

## Metrik terukur vs SketchUp

Semua metrik diukur pada MacBook Apple M4 16 GB. Hasilnya dicatat di `docs/metrics/` per rilis.

| Metrik | Protokol | Target v0.3 | Target v1.0 |
|---|---|---|---|
| Waktu rumah LOD2 oleh pemula | 3 pengguna yang belum pernah memakai kedua tool. Tugas: footprint 10×8 m, tinggi 6 m, atap gable 30°, 1 pintu, 4 jendela. Median waktu. | ≤ 1,5× SketchUp | ≤ 1,0× SketchUp |
| FPS orbit | Adegan 1M dan 10M face (LOD2 hasil batch), jendela 1920×1080, median 10 detik orbit | 1M ≥ 60 FPS | 10M ≥ 30 FPS |
| Waktu buka/simpan | Model 1M face, cold start | ≤ 3 s | ≤ 2 s |
| RAM | Model 1M face setelah dibuka | ≤ 2 GB | ≤ 1,5 GB |
| Presisi koordinat besar | 100 operasi acak (push/pull, move, rotate) di X=430.000, Y=9.150.000 m, dibandingkan hasil yang sama di origin | galat ≤ 1 mm | galat ≤ 0,1 mm |
| Bangunan per jam | (a) manual, (b) template, (c) otomatis + koreksi; 50 footprint campuran | (b) ≥ 60/jam | (c) ≥ 300/jam |
| Validitas ekspor | Persentase bangunan yang lolos val3dity dan cjval | LOD1/LOD2 100% | LOD3 ≥ 98% |

Status "setara" dan "unggul" didefinisikan di [ringkasan riset](../research/ringkasan.md#definisi-metrik-setara-dan-unggul).
