# Tahap 5 — Roadmap

Status: draf 2026-10-29, dikoreksi 2026-11-01 untuk jalur A2. Roadmap ini berbasis **milestone**. Setiap fase selesai bila semua kriteria keluarnya terpenuhi, bukan bila tanggalnya tiba. Waktu kerja manusia fleksibel, jadi estimasi ditulis dalam **jam manusia** (review, uji manual, keputusan, mengarahkan agen) lalu dikonversi ke bulan untuk beberapa ritme kerja.

Semua estimasi adalah **asumsi awal**. Koreksi setelah Fase 0 dengan data nyata (jam tercatat di log sesi).

## Ringkasan fase

| Fase | Tujuan | Jalur A: jam (best/likely/worst) | Jalur B: jam |
|---|---|---|---|
| F0 | Fondasi + spike yang membuktikan risiko terbesar | 15 / 30 / 50 | 40 / 70 / 120 |
| v0.1 | Modeling inti setara SketchUp dasar | 20 / 40 / 70 | 120 / 200 / 350 |
| v0.2 | Geospasial + LOD1 + CityJSON | 40 / 70 / 120 | 40 / 70 / 120 |
| v0.3 | LOD2: atap, semantik, val3dity, CityGML, 3D Tiles | 70 / 120 / 200 | 70 / 120 / 200 |
| v0.4 | LOD3: fasad, komponen, multi-LOD | 60 / 100 / 180 | 60 / 100 / 180 |
| v0.5+ | Paritas SketchUp lainnya, API Python, lalu AI-assist | 100 / 180 / 300 | 100 / 180 / 300 |
| **Total s.d. v0.5** | | **305 / 540 / 920** | **430 / 740 / 1270** |

Konversi kumulatif ke bulan kalender (likely; 1 bulan = 4,33 minggu):

| Sampai | Jalur A @5 j/mgg | A @10 | A @20 | Jalur B @5 | B @10 | B @20 |
|---|---|---|---|---|---|---|
| F0 | 1,4 | 0,7 | 0,3 | 3,2 | 1,6 | 0,8 |
| v0.1 | 3,2 | 1,6 | 0,8 | 12,5 | 6,2 | 3,1 |
| v0.2 | 6,5 | 3,2 | 1,6 | 15,7 | 7,9 | 3,9 |
| v0.3 | 12,0 | 6,0 | 3,0 | 21,2 | 10,6 | 5,3 |
| v0.4 | 16,6 | 8,3 | 4,2 | 25,9 | 12,9 | 6,5 |
| v0.5 | 24,9 | 12,5 | 6,2 | 34,2 | 17,1 | 8,5 |

Rentang ekstrem: jalur A best @20 j/mgg = 3,5 bulan sampai v0.5; jalur B worst @5 j/mgg = 58,7 bulan. Selisih terbesar antara A dan B ada di v0.1, yaitu fondasi yang sudah dibangun Hew.

Konteks jadwal manusia (dari profil): studi di Glasgow mulai September 2026. Ritme renggang (5 j/mgg) dianggap realistis selama semester. Ritme rapat cocok untuk masa libur.

## Koreksi setelah K2 = A2 (2026-11-01)

Jalur yang dipilih adalah **A2** (ADR-000). Tabel di atas memakai kolom jalur A dan B, jadi dibaca sebagai berikut.

| Fase | Jalur A2: jam (best/likely/worst), **asumsi** | Dasar |
|---|---|---|
| F0 sisa | 30 / 50 / 80 | Shell egui + renderer wgpu ber-batching + RTC + origin proyek. Prototipe spike sudah ada (`research/spike-a2-*`). |
| v0.1 | 100 / 140 / 200 | Seluruh tool UI ditulis ulang di atas kernel Hew. Estimasi ±140–200 dari `research/keputusan-k2.md`, dengan sisa F0 dipisah. |
| v0.2–v0.5 | sama dengan kolom jalur A | Fitur geospasial/LOD tidak bergantung jalur. |

Catatan:
- **Jam manusia turun, risiko naik.** Pemilik tidak mereview kode kernel (ADR-000, K2). Jam manusia per fase didominasi keputusan dan uji perilaku, sehingga kemungkinan lebih rendah dari tabel. Gantinya, review silang agen (Gemini) dan test otomatis wajib di setiap PR. Uji beta oleh 3D engineer menjadi kriteria keluar tambahan sejak v0.1.
- Retrospektif jam nyata Fase 0 belum bisa dihitung, karena jam manusia tidak tercatat di log sesi. Mulai F0 sisa, log sesi mencatat jam manusia.

**Sisa pekerjaan F0 untuk A2:**
1. Workspace Cargo: crate Hew `kernel`, `inference`, `tessellate` sebagai dependensi git yang dipin ke satu commit. CI `cargo test` + clippy di 3 OS.
2. Shell egui/eframe dengan viewport wgpu: renderer ber-batching per tile, RTC, edge rendering (#31, #34, #36).
3. Origin proyek: dokumen menyimpan CRS + origin `f64` di atribut `wangungis.*` (ADR-006). Koordinat kernel lokal ≤ ±1.000 km.
4. Golden-image test: kotak di origin vs di UTM 49S harus identik dalam toleransi piksel (sisa kriteria S2).

**Tiga fitur pertama v0.1** (dipilih 2026-11-01; ini yang pertama bisa diuji 3D engineer):
1. Navigasi + seleksi: orbit/pan/zoom, zoom extents, pilih face/edge lewat picking `inference` per tile (#31, #32, #19).
2. Gambar: Line + Rectangle dengan snapping endpoint/midpoint, inference sumbu, dan input angka VCB (#2, #3, #20, #22).
3. Push/Pull + undo/redo lewat `kernel` Hew, dengan pratinjau langsung (#6, #17).

Alasan: ketiganya cukup untuk tugas referensi v0.1 "kotak 5×4×3 m", dan masing-masing menguji satu risiko terukur dari spike (render, inference, kernel).

## Fase 0 — Fondasi dan spike

Tujuan: membuktikan atau membantah risiko terbesar sebelum menulis fitur.

Pekerjaan:
1. Repo git, lisensi sementara, struktur modul (AGENTS.md §4), CI GitHub Actions macOS/Windows/Linux.
2. **Spike S1 — Evaluasi Hew** (ADR-000, 7 kriteria).
3. **Spike S2 — Presisi UTM**: push/pull + move pada X=430.000, Y=9.150.000; bandingkan dengan origin; render RTC tanpa jitter (#36, kedua jalur; jalur A menambahkan RTC bila Hew belum punya).
4. **Spike S3 — Face poligon ↔ Manifold**: boolean pada dua kotak dan satu bangunan L; planaritas setelah kembali ≤ toleransi.
5. **Spike S4 — val3dity di CI**: ekspor satu kubus LOD1 ke CityJSON, jalankan `val3dity` dan `cjval` di ketiga OS.
6. Jalur B saja: half-edge minimal + Push/Pull + inference endpoint/sumbu + render wgpu (matrix #1, #6, #19–#20, #22, #31, #34).

Kriteria keluar:
- CI hijau di 3 OS pada main.
- S1 menghasilkan keputusan A/B tertulis (ADR-000 diperbarui).
- S2: galat ≤ 1 mm setelah 100 operasi; golden-image di origin dan UTM identik dalam toleransi piksel.
- S3: test property 1.000 kasus acak lulus, atau keputusan tertulis untuk mengganti pendekatan.
- S4: val3dity dan cjval lulus di CI.
- ADR-001..007 berstatus Accepted atau direvisi.

Rilis: tidak ada rilis publik; repo publik dengan README "pre-alpha".

## v0.1 — Modeling inti

Fitur: matrix #1–#14, #17, #19–#28, #31–#34, #42–#46, #49, #83, #89–#90, #97, #100.
Jalur A: sebagian besar sudah ada di Hew. Tugasnya: verifikasi dengan test kita, tambahkan CRS/origin di format, dan siapkan crate `lod`/`geo` sebagai titik ekstensi. Pekerjaan baru di jalur A: #14 auto-split face (tidak ada di Hew, kompleksitas L), ekspor OBJ (#90, Hew hanya STL), dan item `Hew: ?` yang belum terverifikasi. Estimasi 40 jam likely kemungkinan rendah; dikoreksi setelah spike S1.

Kriteria keluar:
- Tugas referensi "kotak 5×4×3 m dengan pintu, lalu salin 10× dalam array" selesai tanpa error di 3 OS.
- Undo/redo dan replay deterministik untuk setiap tool; test property tidak menemukan invariant rusak dalam 10.000 kasus.
- Save → load → save menghasilkan file byte-identik.
- Rilis open source `v0.1.0`: binary macOS/Windows/Linux, catatan instalasi tanpa signing (lihat risiko R8).

## v0.2 — Geospasial + LOD1

Fitur: #30, #37, #48, #57–#62, #65–#66, #82, #84–#85, #91, #94.

Kriteria keluar:
- Impor 10.000 footprint (GeoJSON/Shapefile) + atribut tinggi menjadi LOD1 dalam ≤ 30 detik di M4.
- Proyek dengan EPSG:32748/32749 (UTM zona Indonesia) dan EPSG:28992 (NL) bekerja tanpa drift (S2 sebagai test regresi).
- Ekspor CityJSON 2.0 lolos `cjval` dan `val3dity` 100%.
- Round-trip tile 3D BAG: impor → simpan → ekspor tanpa kehilangan atribut atau semantik.
- FPS orbit ≥ 60 pada 1M face.
- Rilis `v0.2.0`.

## v0.3 — LOD2

Fitur: #15–#16, #63, #67–#75, #86–#88; #72 roof fitting minimal untuk atap planar sederhana.

Kriteria keluar:
- Must: template flat, shed, gable, dan hip pada footprint persegi (#67–#68).
- Should: atap pada footprint L, T, dan U (#69), mansard (#70), dan overhang (#71).
- Klasifikasi semantik otomatis benar pada 100% kasus uji.
- Validasi live menampilkan kode error val3dity yang sama dengan validator CLI.
- Ekspor CityGML 2.0 dan 3D Tiles 1.1; LOD2 lolos val3dity 100%.
- Metrik "rumah LOD2 oleh pemula" ≤ 1,5× SketchUp.
- Rilis `v0.3.0` — **rilis pertama yang dipromosikan ke komunitas geospasial** (OSGeo, forum CityJSON, 3D BAG).

## v0.4 — LOD3

Fitur: #50, #64, #76–#81, roof fitting dari DSM/point cloud (#72 penuh).

Kriteria keluar:
- Opening jendela/pintu sebagai komponen; template fasad per lantai.
- Satu bangunan menyimpan LOD1/LOD2/LOD3; generalisasi otomatis LOD3→LOD2→LOD1 lolos val3dity.
- LOD3 lolos val3dity ≥ 98% pada set uji 50 bangunan.
- Rilis `v0.4.0`.

## v0.5+ — Paritas lainnya, lalu AI-assist

Fitur: #18, #29, #35, #38–#40, #47, #51–#56, #92, #95, #98–#99; lalu #96 (segmentasi ONNX lokal). Fitur Won't (#41, #93) dievaluasi ulang setelah v1.0.
Kriteria keluar per sub-rilis ditetapkan saat v0.4 selesai berdasarkan data pengguna.

## Agenda per fase (ritual tetap)

| Kapan | Kegiatan |
|---|---|
| Awal fase | Pilih fitur dari matrix; tulis/perbarui SPEC.md modul yang tersentuh |
| Setiap sesi | Protokol skill → satu tugas → log sesi |
| Setiap 2 minggu | Manusia membaca ulang SPEC.md modul inti; perbarui kolom Status di feature matrix |
| Akhir fase | `/ponytail-audit`, `/ponytail-debt`, catat metrik (tokei, cargo tree, waktu build, FPS, val3dity); retrospektif 10 baris; koreksi estimasi fase berikut |
