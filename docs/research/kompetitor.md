# Riset Tahap 0.A + 0.B — Kompetitor, Alur LOD, Pain Point

Status: draf Sesi 2 (2026-10-28). Tautan diakses 2026-10-28.

## Metode dan tingkat keyakinan

- Sel matriks untuk **fitur LOD/geospasial (baris 37–52)** didasarkan pada sumber yang dikutip di §2.
- Sel untuk modeling umum, organisasi, dan I/O berasal dari dokumentasi produk yang umum diketahui. Sel ini **asumsi tingkat sedang**: cukup untuk menentukan prioritas, tetapi harus dicek sebelum dipakai sebagai klaim publik ("setara/unggul").
- `?` = belum diverifikasi. Jangan anggap `?` sebagai "tidak ada".

Legenda: `✓` native · `P` parsial atau lewat plugin/add-on · `–` tidak ada · `?` belum diverifikasi.

Kolom: **SU** SketchUp · **BL** Blender (+BlenderGIS, Archipack, Up3date) · **RH** Rhino/Grasshopper · **FC** FreeCAD · **RV** Revit · **BB** Bonsai (BlenderBIM) · **CE** CityEngine · **HO** Houdini · **S3** Shapr3D · **PL** Plasticity · **HW** Hew.

## 1. Matriks fitur (66 fitur × 11 tool)

| # | Fitur | SU | BL | RH | FC | RV | BB | CE | HO | S3 | PL | HW |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| | **Modeling inti** | | | | | | | | | | | |
| 1 | Line/polyline berbasis klik | ✓ | P | ✓ | ✓ | ✓ | P | ✓ | P | ✓ | ✓ | ✓ |
| 2 | Rectangle | ✓ | P | ✓ | ✓ | ✓ | P | ✓ | P | ✓ | ✓ | ✓ |
| 3 | Circle / Arc | ✓ | P | ✓ | ✓ | ✓ | P | ? | P | ✓ | ✓ | ✓ |
| 4 | Push/Pull face | ✓ | P | ✓ | P | P | P | ✓ | P | ✓ | ✓ | ✓ |
| 5 | Follow Me / sweep | ✓ | P | ✓ | ✓ | ✓ | P | – | ✓ | ✓ | ✓ | ✓ |
| 6 | Offset | ✓ | P | ✓ | ✓ | ✓ | P | ✓ | ✓ | ✓ | ✓ | ✓ |
| 7 | Move/Rotate/Scale + snapping | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| 8 | Tape measure / guide | ✓ | P | P | P | ✓ | P | – | – | P | P | ✓ |
| 9 | Soften/smooth edge | ✓ | ✓ | P | – | – | P | – | ✓ | – | – | ? |
| 10 | Boolean / solid tools | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | P | ✓ | ✓ | ✓ | ✓ |
| 11 | Intersect faces | ✓ | ✓ | ✓ | ✓ | – | P | – | ✓ | P | P | P |
| 12 | "Sticky geometry" (face menyatu otomatis) | ✓ | – | – | – | – | – | – | – | – | – | – (sengaja) |
| 13 | Solid/watertight sejak dibuat | – | – | ✓ | ✓ | ✓ | ✓ | P | P | ✓ | ✓ | ✓ |
| 14 | Riwayat parametrik | – | P | P | ✓ | ✓ | P | ✓ | ✓ | ✓ | – | – |
| 15 | Permukaan NURBS/lengkung | – | P | ✓ | ✓ | P | P | – | ✓ | ✓ | ✓ | – |
| | **Inference** | | | | | | | | | | | |
| 16 | Snap endpoint/midpoint/on-edge | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | P | ✓ | ✓ | ✓ |
| 17 | Menggambar langsung di face | ✓ | P | P | P | ✓ | P | P | – | ✓ | ✓ | ✓ |
| 18 | Kunci sumbu (Shift/panah/X-Y-Z) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ? | ✓ | P | P | ✓ |
| 19 | Inference paralel/tegak lurus | ✓ | P | ✓ | ✓ | ✓ | P | ? | – | ✓ | P | ✓ |
| 20 | Input numerik saat menggambar (VCB) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | P | P | ✓ | ✓ | ✓ |
| | **Organisasi** | | | | | | | | | | | |
| 21 | Groups | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | P | ✓ | ✓ | ✓ | ✓ |
| 22 | Components/instance (edit sekali) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | P | ✓ | P | P | ✓ |
| 23 | Tags/layers | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| 24 | Scenes/saved views | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | P | P | ✓ |
| 25 | Outliner | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| 26 | Styles (gaya edge/sketsa) | ✓ | P | ✓ | P | ✓ | P | – | – | P | P | ? |
| | **Material dan visual** | | | | | | | | | | | |
| 27 | Material / paint bucket | ✓ | ✓ | ✓ | P | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| 28 | Posisi tekstur | ✓ | ✓ | ✓ | P | P | ✓ | ✓ | ✓ | P | – | ? |
| 29 | PBR di viewport | ? | ✓ | ✓ | – | P | ✓ | ✓ | ✓ | P | P | – |
| 30 | Bayangan matahari tergeolokasi | ✓ | P | ✓ | P | ✓ | ✓ | ✓ | ✓ | – | – | – |
| 31 | Section plane | ✓ | P | ✓ | ✓ | ✓ | ✓ | – | P | ✓ | ? | P |
| 32 | Render fotorealistik | P | ✓ | ✓ | P | ✓ | ✓ | P | ✓ | P | P | – |
| 33 | Hidden-line / edge gaya CAD | ✓ | P | ✓ | ✓ | ✓ | ✓ | – | – | ✓ | ✓ | ✓ |
| | **Dokumentasi** | | | | | | | | | | | |
| 34 | Dimensi dan teks | ✓ | P | ✓ | ✓ | ✓ | ✓ | – | – | ✓ | – | ✓ |
| 35 | Lembar gambar 2D (setara LayOut) | ✓ | P | ✓ | ✓ | ✓ | ✓ | – | – | ✓ | – | P |
| 36 | Cetak/PDF berskala | ✓ | P | ✓ | ✓ | ✓ | ✓ | – | – | ✓ | – | ✓ |
| | **Geospasial** | | | | | | | | | | | |
| 37 | Geolokasi / CRS | P | P | P | ? | ✓ | ✓ | ✓ | P | – | – | – |
| 38 | Presisi koordinat besar (UTM) tanpa drift | ? | ? | P | ? | P | P | ✓ | P | – | – | – |
| 39 | Terrain dari DEM/GeoTIFF | P | P | P | ? | ✓ | P | ✓ | ✓ | – | – | – |
| 40 | Impor footprint SHP/GeoJSON | P | P | P | ? | P | ? | ✓ | P | – | – | – |
| 41 | Impor OSM | P | P | P | ? | – | ? | ✓ | ✓ | – | – | – |
| 42 | Impor point cloud | P | P | ✓ | P | ✓ | P | – | ✓ | – | – | – |
| | **Alat LOD** | | | | | | | | | | | |
| 43 | LOD1: batch extrude footprint | P | P | P | – | – | – | ✓ | ✓ | – | – | – |
| 44 | Template atap (gable/hip/shed/…) | P | P | P | ✓ | ✓ | P | ✓ | P | – | – | – |
| 45 | Roof fitting dari DSM/point cloud | – | – | – | – | – | – | – | P | – | – | – |
| 46 | Template fasad / komponen LOD3 | P | P | P | P | ✓ | ✓ | ✓ | ✓ | – | – | P |
| 47 | Permukaan semantik CityGML (Wall/Roof/Ground) | P | P | P | – | – | – | – | – | – | – | – |
| 48 | Impor/ekspor CityJSON | – | P | P | – | – | – | – | – | – | – | – |
| 49 | Ekspor CityGML | P | – | – | – | – | – | – | – | – | – | – |
| 50 | Validasi geometri (watertight; setara val3dity) | P | P | P | ✓ | – | P | – | P | P | P | ✓ |
| 51 | Multi-LOD dalam satu model | – | – | – | – | P | P | P | – | – | – | – |
| 52 | Ekspor 3D Tiles | – | ? | – | – | – | – | P (I3S) | – | – | – | – |
| | **BIM ringan** | | | | | | | | | | | |
| 53 | Ekspor IFC | ✓ | P | P | ✓ | ✓ | ✓ | ✓ | – | ? | – | – |
| 54 | Edit IFC native | – | – | – | ✓ | – | ✓ | – | – | – | – | – |
| | **I/O umum** | | | | | | | | | | | |
| 55 | glTF | ? | ✓ | ✓ | ✓ | P | ✓ | ✓ | ✓ | ? | – | ✓ |
| 56 | OBJ/STL | ✓ | ✓ | ✓ | ✓ | P | ✓ | ✓ | ✓ | ✓ | ✓ | P (STL saja) |
| 57 | DXF/DWG | ✓ | P | ✓ | ✓ | ✓ | P | ✓ | P | ✓ | – | – |
| 58 | USD/USDZ | P | ✓ | ✓ | – | – | – | ✓ | ✓ | ✓ | – | ✓ |
| 59 | Impor .skp | ✓ | P | ✓ | P | ✓ | – | ? | – | ? | – | ✓ |
| | **Ekstensibilitas** | | | | | | | | | | | |
| 60 | API scripting | ✓ Ruby | ✓ Python | ✓ Python/C# | ✓ Python | ✓ C# | ✓ Python | ✓ CGA/Python | ✓ VEX/Python | – | – | P (CLI/MCP) |
| 61 | Pemrograman visual | – | ✓ | ✓ | P | ✓ | P | P | ✓ | – | – | – |
| 62 | Marketplace ekstensi | ✓ | ✓ | ✓ | ✓ | ✓ | P | P | P | – | – | – |
| | **Platform dan bisnis** | | | | | | | | | | | |
| 63 | Linux | – | ✓ | – | ✓ | – | ✓ | – | ✓ | – | ? | ✓ |
| 64 | Open source | – | ✓ | – | ✓ | – | ✓ | – | – | – | – | ✓ |
| 65 | Gratis | P | ✓ | – | ✓ | – | ✓ | – | P | P | P | ✓ |
| 66 | Versi browser | ✓ | – | – | – | – | – | – | – | – | – | ✓ |

Ringkasan matriks:
- Baris 43–52 (alat LOD) hampir kosong di semua tool. CityEngine paling kuat untuk LOD1–LOD3 prosedural, tetapi tidak mengekspor CityGML, tidak open source, dan tidak punya UX gambar-tangan gaya SketchUp.
- Baris 1–25 (UX SketchUp) sudah dipenuhi Hew. Hew kosong di seluruh blok geospasial dan LOD.
- Celah yang belum diisi siapa pun: **UX gambar-tangan gaya SketchUp + semantik CityGML + validasi val3dity + CRS/koordinat besar, dalam satu tool open source.**

## 2. Alur LOD nyata per tool (sumber)

| Tool | Alur LOD saat ini | Sumber |
|---|---|---|
| SketchUp | Add Location untuk geolokasi; konteks situs butuh langganan minimal Go. Ekspor CityGML lewat plugin berbayar CityEditor (butuh Pro; trial dilaporkan memblokir ekspor) atau plugin open source GEORES (terakhir dilaporkan jalan di SU 2019). CityEditor punya georeferensi sendiri, terpisah dari Add Location. | [SketchUp Help](https://help.sketchup.com/zh-hant-tw/sketchup-web/how-use-add-location), Tygron wiki (tautan kini 404; dikutip dari hasil pencarian 2026-10-28), [forum: plugin CityGML](https://forums.sketchup.com/t/citygml-plugins-for-sketchup/24921), [CityEditor PDF](https://assets.3dis.de/CityEditor_en.pdf) |
| Blender | Up3date: impor/edit/ekspor CityJSON v1.0 dengan semantik; repo asli diarsipkan Juni 2022. CityGML harus dikonversi dulu ke CityJSON. BlenderGIS: extrude bangunan OSM; tidak memakai informasi CityJSON. Archipack: atap, dinding, jendela parametrik (GPL). | [Up3date](https://github.com/cityjson/Up3date), [cityjson.org](https://cityjson.org/news/2019/11/25/hello-blender), [OSArch](https://community.osarch.org/discussion/comment/4998/), [Archipack wiki](https://wiki.osarch.org/archipack/) |
| Rhino/Grasshopper | Tidak ada importer CityGML publik untuk Grasshopper; solusi forum: FZK Viewer → STL → Rhino, atau AntFarm. RhinoCityJSON (TU Delft) membaca CityJSON dengan semantik, fokus pada data 3D BAG. | [McNeel forum](https://discourse.mcneel.com/t/citygml-cityjason-3d-in-grasshopper/164717), [RhinoCityJSON](https://github.com/cityjson/RhinoCityJSON), [TU Delft digipedia](https://digipedia.tudelft.nl/software/rhinocityjson/) |
| FreeCAD | IFC native sejak 1.0. Tidak ditemukan importer CityGML/CityJSON bawaan. | [FreeCAD wiki](https://wiki.freecad.org/Arch_IFC), [tutorial NativeIFC](https://yorik.uncreated.net/blog/2025/002-nativeifc-tutorial) |
| Revit / InfraWorks | Revit memakai shared coordinates dan survey point; pengguna melaporkan model salah posisi atau geometri rusak pada koordinat besar saat dibawa ke InfraWorks. InfraWorks dapat mengimpor CityGML. | [Autodesk forum](https://forums.autodesk.com/t5/infraworks-forum/troubles-with-revit-file-import-into-infrawroks-360/m-p/6942239), [InfraWorks help](https://help.autodesk.com/cloudhelp/2015/ENU/InfraWorks-UserHelp/files/GUID-6FDFA5CF-9CB0-47B8-B77F-656B9745186A.htm) |
| CityEngine | Impor SHP, FGDB, OSM, IFC, dll.; ekspor IFC (2x3), glTF, USD, SLPK (I3S); CityGML tidak ada di daftar format resmi. Fitur ditentukan tingkat lisensi. Impor CityGML di ekosistem Esri lewat toolset ArcGIS Pro + Data Interoperability Extension. | [Export](https://doc.arcgis.com/en/cityengine/latest/help/help-export-models-overview.htm), [Import](https://doc.arcgis.com/en/cityengine/latest/help/help-import-dialog.htm), [Esri blog](https://www.esri.com/arcgis-blog/products/arcgis-pro/3d-gis/import3dcitydatainarcgis) |
| Houdini | SideFX Labs: OSM Importer + Building Generator + Road Generator untuk blok kota prosedural. Tidak ada keluaran semantik CityGML. | [SideFX tutorial](https://www.sidefx.com/tutorials/city-building-with-osm-data/), [OSM Importer](https://sidefx.com/tutorials/osm-importer/) |
| Shapr3D / Plasticity | Modeler solid berbasis Parasolid dengan push/pull langsung; tidak ada fitur geospasial. Shapr3D berlangganan; Plasticity lisensi sekali bayar (175/299 USD). | [Shapr3D](https://www.shapr3d.com/product/3d-modeling), [Plasticity](https://www.plasticity.xyz/buy) |
| 3DCityDB | Impor/ekspor CityGML 2.0/1.0 dengan filter LoD (basis data, bukan modeler). | [3DCityDB docs](https://3dcitydb-docs.readthedocs.io/en/release-v4.1/impexp/citygml-export.html) |
| Pipeline otomatis | `tyler`: CityJSON → 3D Tiles 1.1. `cjval`: validasi skema. val3dity: validasi geometri ISO 19107. | lihat [teknologi.md](teknologi.md) §5 |

Harga referensi SketchUp 2026: Go 129 USD/tahun, Pro 399 USD/tahun, Studio ±819 USD/tahun (sumber sekunder; angka antar-sumber berbeda) [myarchitectai](https://www.myarchitectai.com/blog/sketchup-pricing).

## 3. Pain point pengguna (0.B)

Sumber bahan mentah: [_raw-painpoints.md](_raw-painpoints.md). Bahan ini dikumpulkan oleh agy (Gemini) dan berisi 38 butir dengan 80 URL unik.

Verifikasi oleh Claude (2026-10-29):
- 68 dari 80 URL mengembalikan HTTP 200. Yang tersisa: 10 URL Reddit dan alternativeto.net menolak bot (403), 1 timeout (research.tudelft.nl). Isi URL-URL ini belum dicek.
- Diperiksa ke sumbernya:
  - A2 sesuai: SketchUp gagal membentuk face pada segmen ±0,001 inci; workaround "Dave Method" ([forum](https://forums.sketchup.com/t/the-dave-method/49720)).
  - A3 sesuai: penjualan lisensi perpetual Classic berhenti 4 November 2020 ([SketchUp Help](https://help.sketchup.com/en/license-types-and-pricing)).
  - A5 **dilebih-lebihkan**: thread yang dikutip melaporkan koordinat DXF salah dan geometri hilang/terpotong jauh dari origin, bukan "jitter" ([forum](https://forums.sketchup.com/t/real-world-coordinates/340782)). Saran komunitas: bekerja dalam ±1 km dari origin dan menyimpan origin proyek terpisah. Hal ini tetap mendukung USP presisi.
- Butir lain belum dicek satu per satu. Anggap butir-butir itu sebagai indikasi, bukan bukti.

| # | Pain point SketchUp | Dampak ke proyek | Sumber utama |
|---|---|---|---|
| A1 | Mesin modeling berjalan di satu core CPU; model besar melambat | Batch LOD dan validasi paralel (arsitektur §5) | [forum](https://forums.sketchup.com/t/how-do-you-keep-large-sketchup-models-running-smoothly/350331) |
| A2 | Face/edge < ±0,001 inci gagal dibuat ("Dave Method", skala 1000×) | Toleransi terpusat + `f64`; test edge sangat pendek | [forum](https://forums.sketchup.com/t/the-dave-method/49720) |
| A3 | Hanya langganan sejak 4 Nov 2020; SketchUp Make dihentikan | Gratis dan open source | [SketchUp Help](https://help.sketchup.com/en/license-types-and-pricing) |
| A4 | Ekstensi Ruby C rusak saat upgrade Ruby | API Python berversi, proses terpisah (ADR-007) | [forum](https://forums.sketchup.com/t/important-upcoming-ruby-update/348793) |
| A5 | Koordinat dunia nyata: geometri hilang/terpotong jauh dari origin; saran "tetap dalam 1 km" | CRS + origin `f64` + RTC (ADR-003) | [forum](https://forums.sketchup.com/t/real-world-coordinates/340782) |
| A6 | Ekspor IFC/CityGML lemah, bergantung plugin | CityGML/CityJSON kelas satu | lihat §2 |
| A7–A10 | Group "solid" yang ternyata tidak solid; Solid Tools dan boolean rapuh; Intersect Faces gagal | Watertight sejak dibuat, validasi live, Manifold | [forum](https://forums.sketchup.com/t/solid-inspector-shows-good-but-it-is-not-solid/206705), [forum](https://forums.sketchup.com/t/trouble-with-intersect-faces/35124) |
| A11 | Sticky geometry: geometri lepas menempel tanpa disengaja | Pertimbangkan perilaku "solid per objek" ala Hew untuk bangunan | `_raw-painpoints.md` |
| A12 | Tidak ada versi Linux | Build Linux di CI sejak hari pertama | [forum](https://forums.sketchup.com/t/how-to-use-sketchup-on-linux/131201) |
| A13 | LayOut membeku pada viewport vektor | Dokumen 2D ditunda (v0.5+), jangan janji paritas cepat | `_raw-painpoints.md` |

| # | Pain point alur LOD | Dampak ke proyek | Sumber utama |
|---|---|---|---|
| B1–B2 | 3dfier dan roofer (pipeline 3D BAG) kuat untuk batch LOD1–LOD2.2, tetapi tidak punya editor interaktif untuk atap yang gagal | Posisi produk: **editor koreksi** di atas hasil pipeline otomatis, bukan pengganti pipeline | [3dfier](https://tudelft3d.github.io/3dfier/generate_lod1.html), [roofer](https://innovation.3dbag.nl/roofer/), [3D BAG docs](https://docs.3dbag.nl/en/) |
| B3 | Model dari tool interaktif sering gagal val3dity: face non-planar (203/204), shell tidak tertutup (302), normal terbalik (307) | Validasi live dengan kode error val3dity yang sama | [val3dity errors](https://val3dity.readthedocs.io/2.6.0/errors/) |
| B9 | Opening LOD3 (jendela/pintu) dimodelkan tangan, menjadi bottleneck terbesar | Template opening parametrik (feature #76–77) | `_raw-painpoints.md` |

| # | Hew di komunitas | Dampak | Sumber |
|---|---|---|---|
| C1–C4 | Pengumuman rilis Hew di r/Sketchup menunjukkan minat pada alternatif gratis lintas platform. Fokus Hew: cetak 3D dan pertukangan kayu; tidak ada GIS | Mendukung jalur A: komunitas UX sudah ada; niche geospasial tidak bersaing dengan fokus Hew | Reddit (403 untuk bot; belum dicek isinya), [GitHub](https://github.com/hew3d/hew) |

Temuan baru yang mengubah positioning (dimasukkan ke §4 butir 8): pipeline otomatis sudah menyelesaikan mayoritas bangunan. Celah terbesar ada pada **koreksi interaktif** hasil yang gagal dan pada **LOD3**.

## 4. Peluang diferensiasi (sementara)

1. **CityGML/CityJSON sebagai format kelas satu**, bukan plugin berbayar. Saat ini ekspor CityGML dari SketchUp bergantung pada plugin berbayar atau plugin yang tidak terawat (§2).
2. **Satu georeferensi** (CRS + origin) yang dipakai semua alat. Saat ini SketchUp punya dua mekanisme terpisah (Add Location vs CityEditor), dan Revit/InfraWorks sering salah posisi (§2).
3. **Presisi koordinat UTM tanpa drift** sebagai fitur yang diuji otomatis (§ teknologi RTC).
4. **Validasi bawaan** (watertight, planaritas, val3dity) saat menggambar, bukan setelah ekspor.
5. **LOD1 batch + template atap LOD2 + roof fitting** dalam UX gambar-tangan. CityEngine dan Houdini bersifat prosedural; Hew/SketchUp bersifat manual; tidak ada yang menggabungkan keduanya.
6. **Multi-LOD dalam satu model** dengan konversi antar-LOD (baris 51 kosong di semua tool).
7. **Open source, gratis, Linux** — hanya Blender, FreeCAD, Bonsai, dan Hew yang memenuhi; tidak satu pun punya alur LOD lengkap.
8. **Editor koreksi untuk hasil pipeline otomatis** (3dfier, roofer/3D BAG): buka CityJSON, tandai bangunan yang gagal validasi, perbaiki dengan tool gaya SketchUp, lalu simpan tanpa kehilangan semantik (§3 B1–B3).
