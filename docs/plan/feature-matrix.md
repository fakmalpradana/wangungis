# Tahap 2 — Feature Matrix

Status: draf 2026-10-28. Dokumen ini dilacak sepanjang proyek: kolom **Status** diperbarui di PR yang menyelesaikan fitur.

Legenda:
- **SU**: ada di SketchUp (✓/P/–). **Hew**: ada di Hew v1.3 (✓/P/–/?). Isi kolom Hew menentukan "verifikasi saja" di jalur A.
- **Prio**: M (must), S (should), C (could), W (won't sebelum v1.0).
- **Kompl**: S, M, L, XL.
- **Jenis**: T = tulis sendiri (inti diferensiasi, AGENTS.md §5), R = reuse library, T+R = campuran.
- **Status**: belum / parsial / setara / unggul ([definisi](../research/ringkasan.md#definisi-metrik-setara-dan-unggul)).
- **Test**: U unit, P property-based/fuzz, G golden-image, B benchmark, V val3dity/cjval, E end-to-end replay.

## Modeling inti

| # | Fitur | SU | Hew | Prio | Kompl | Rilis | Library | Test | Jenis | Status |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | Half-edge mesh + validator invariant | – | ✓ | M | XL | F0 | — | U P | T | belum |
| 2 | Line tool | ✓ | ✓ | M | M | 0.1 | — | U E | T | parsial |
| 3 | Rectangle (+ rotated) | ✓ | ✓ | M | S | 0.1 | — | U E | T | parsial |
| 4 | Circle / Polygon | ✓ | ✓ | M | S | 0.1 | — | U E | T | parsial |
| 5 | Arc (2-point, 3-point, pie) | ✓ | ✓ | M | M | 0.1 | — | U E | T | parsial |
| 6 | Push/Pull (+ live preview) | ✓ | ✓ | M | XL | F0/0.1 | — | U P E | T | parsial |
| 7 | Push/Pull dengan Ctrl (buat face baru) | ✓ | ✓ | S | S | 0.1 | — | U P | T | parsial |
| 8 | Follow Me | ✓ | ✓ | C | L | 0.1 | — | U P | T | parsial |
| 9 | Offset | ✓ | ✓ | S | M | 0.1 | i_overlay/Clipper2 | U P | T+R | parsial |
| 10 | Move (+ copy, array) | ✓ | ✓ | M | M | 0.1 | — | U E | T | parsial |
| 11 | Rotate (+ copy, radial array) | ✓ | P | M | M | 0.1 | — | U E | T | parsial |
| 12 | Scale (+ uniform, dari tengah) | ✓ | ✓ | M | M | 0.1 | — | U | T | parsial |
| 13 | Eraser (+ hide/soften) | ✓ | ? | M | S | 0.1 | — | U P | T | parsial |
| 14 | Auto-split face saat garis memotong face | ✓ | – | M | L | 0.1 | robust | U P | T | parsial |
| 15 | Intersect Faces | ✓ | P | S | XL | 0.3 | Manifold | U P | T+R | belum |
| 16 | Boolean solid (union/subtract/intersect) | ✓ | ✓ | S | L | 0.3 | Manifold | U P V | R | belum |
| 17 | Undo/redo semua operasi | ✓ | ✓ | M | L | F0/0.1 | — | U P E | T | parsial |
| 18 | Mirror / flip | ✓ | – | C | S | 0.5 | — | U | T | belum |

## Inference dan input

| # | Fitur | SU | Hew | Prio | Kompl | Rilis | Library | Test | Jenis | Status |
|---|---|---|---|---|---|---|---|---|---|---|
| 19 | Ray picking + BVH | ✓ | ✓ | M | M | F0 | bvh crate (cek ADR) | U B | T+R | parsial |
| 20 | Snap endpoint / midpoint | ✓ | ✓ | M | M | F0 | — | U | T | parsial |
| 21 | Snap on-edge / on-face / center | ✓ | ✓ | M | M | 0.1 | — | U | T | parsial |
| 22 | Inference sumbu (merah/hijau/biru) | ✓ | ✓ | M | M | F0 | — | U | T | parsial |
| 23 | Kunci sumbu (Shift, tombol panah) | ✓ | ✓ | M | S | 0.1 | — | U E | T | parsial |
| 24 | Inference paralel / tegak lurus (magenta) | ✓ | P | M | M | 0.1 | — | U P | T | parsial |
| 25 | Prioritas snapping + tooltip | ✓ | ✓ | M | M | 0.1 | — | U | T | parsial |
| 26 | VCB: panjang, jarak, sudut, jumlah array | ✓ | ✓ | M | M | 0.1 | — | U E | T | parsial |
| 27 | Unit dan presisi tampilan (m, cm, mm, ft) | ✓ | ✓ | M | S | 0.1 | — | U | T | parsial |
| 28 | Tape Measure + guide line/point | ✓ | ✓ | S | M | 0.1 | — | U | T | parsial |
| 29 | Protractor | ✓ | ? | C | S | 0.5 | — | U | T | belum |
| 30 | Inference ke titik di koordinat UTM tanpa galat | – | – | M | M | 0.2 | — | U P | T | belum |

## Navigasi dan tampilan

| # | Fitur | SU | Hew | Prio | Kompl | Rilis | Library | Test | Jenis | Status |
|---|---|---|---|---|---|---|---|---|---|---|
| 31 | Orbit / pan / zoom (mouse tengah, scroll) | ✓ | ✓ | M | M | F0 | — | E | T | parsial |
| 32 | Zoom extents / zoom window | ✓ | ✓ | M | S | 0.1 | — | U | T | parsial |
| 33 | Proyeksi perspektif / paralel, view standar | ✓ | ✓ | M | S | 0.1 | — | G | T | parsial |
| 34 | Edge rendering gaya CAD (profil, garis) | ✓ | ✓ | M | L | F0/0.1 | wgpu/three.js | G | T+R | parsial |
| 35 | Face style (shaded, textured, X-ray, wireframe) | ✓ | ? | S | M | 0.5 | — | G | T | belum |
| 36 | Rendering RTC untuk koordinat besar | – | – | M | M | F0 | — | G P | T | belum |
| 37 | Culling + LOD rendering + instancing | P | ? | M | L | 0.2 | — | B | T | belum |
| 38 | Shadows tergeolokasi (tanggal/jam) | ✓ | – | S | L | 0.5 | solar position (asumsi crate) | G U | T+R | belum |
| 39 | Section plane (+ fill) | ✓ | P | S | L | 0.5 | — | G | T | belum |
| 40 | Styles (preset edge/face/background) | ✓ | ? | C | M | 0.5 | — | G | T | belum |
| 41 | Fog / watermark | ✓ | – | W | S | — | — | — | T | belum |

## Organisasi

| # | Fitur | SU | Hew | Prio | Kompl | Rilis | Library | Test | Jenis | Status |
|---|---|---|---|---|---|---|---|---|---|---|
| 42 | Groups (edit di dalam konteks) | ✓ | ✓ | M | M | 0.1 | — | U E | T | parsial |
| 43 | Components (definisi + instance) | ✓ | ✓ | M | L | 0.1 | — | U P | T | parsial |
| 44 | Make unique / explode | ✓ | ? | S | S | 0.1 | — | U | T | parsial |
| 45 | Tags (visibilitas, warna) | ✓ | ✓ | M | S | 0.1 | — | U | T | parsial |
| 46 | Outliner | ✓ | ✓ | M | M | 0.1 | — | E | T | belum |
| 47 | Scenes (kamera, tag, style tersimpan) | ✓ | ✓ | S | M | 0.5 | — | U | T | belum |
| 48 | Entity Info / atribut objek | ✓ | ? | M | S | 0.2 | — | U | T | belum |

## Material

| # | Fitur | SU | Hew | Prio | Kompl | Rilis | Library | Test | Jenis | Status |
|---|---|---|---|---|---|---|---|---|---|---|
| 49 | Paint Bucket + library material | ✓ | ✓ | M | M | 0.1 | — | U P G | T | parsial |
| 50 | Tekstur gambar + posisi tekstur | ✓ | ✓ | S | M | 0.4 | image crate | G | T+R | belum |
| 51 | Material PBR dasar | P | – | C | M | 0.5 | — | G | T | belum |
| 52 | Tekstur fasad dari foto (rectify) | – | – | C | L | 0.5+ | — | G | T | belum |

## Dimensi dan dokumentasi

| # | Fitur | SU | Hew | Prio | Kompl | Rilis | Library | Test | Jenis | Status |
|---|---|---|---|---|---|---|---|---|---|---|
| 53 | Dimension tool | ✓ | ✓ | S | M | 0.5 | — | G | T | belum |
| 54 | Text / label / 3D text | ✓ | ✓ | C | M | 0.5 | font crate | G | T+R | belum |
| 55 | Dokumen 2D berskala (setara LayOut) | ✓ | P | C | XL | 0.5+ | — | G | T | belum |
| 56 | Ekspor PDF/SVG | ✓ | ✓ | C | M | 0.5+ | — | G | R | belum |

## Geospasial

| # | Fitur | SU | Hew | Prio | Kompl | Rilis | Library | Test | Jenis | Status |
|---|---|---|---|---|---|---|---|---|---|---|
| 57 | Proyek ber-CRS (EPSG) + origin `f64` | P | – | M | M | 0.2 | PROJ | U P | T+R | belum |
| 58 | Reprojection saat impor | – | – | M | M | 0.2 | PROJ | U | R | belum |
| 59 | Impor GeoJSON | – | – | M | S | 0.2 | geojson crate | U | R | belum |
| 60 | Impor Shapefile | – | – | M | S | 0.2 | shapefile crate / GDAL | U | R | belum |
| 61 | Impor GeoTIFF/DEM sebagai terrain | P | – | S | M | 0.2 | GDAL/tiff crate | U G | R | belum |
| 62 | Impor OSM | P | – | C | M | 0.2 | osmpbf crate (asumsi) | U | R | belum |
| 63 | Basemap/ortofoto sebagai referensi | P | P | C | M | 0.3 | GDAL | G | R | belum |
| 64 | Impor point cloud (LAS/LAZ) | P | – | C | L | 0.4 | las crate / PDAL | U B | R | belum |

## LOD tools

| # | Fitur | SU | Hew | Prio | Kompl | Rilis | Library | Test | Jenis | Status |
|---|---|---|---|---|---|---|---|---|---|---|
| 65 | LOD1 batch: extrude footprint dari atribut tinggi | – | – | M | M | 0.2 | i_overlay | U P V | T+R | belum |
| 66 | Pembersihan footprint (self-intersection, hole) | – | – | M | M | 0.2 | i_overlay | U P | T+R | belum |
| 67 | Atap flat / shed | P | – | M | M | 0.3 | — | U P V | T | belum |
| 68 | Atap gable / hip pada footprint persegi | P | – | M | L | 0.3 | straight skeleton (asumsi crate) | U P V | T | belum |
| 69 | Atap hip/gable pada footprint L/T/U (straight skeleton) | – | – | S | XL | 0.3 | straight skeleton | U P V | T | belum |
| 70 | Atap mansard | – | – | S | L | 0.3 | — | U P V | T | belum |
| 71 | Overhang atap | – | – | S | M | 0.3 | — | U P V | T | belum |
| 72 | Roof fitting dari DSM/point cloud | – | – | S | XL | 0.3/0.4 | data raster/point cloud dari `io` (tiff/las crate) | U B V | T | belum |
| 73 | Semantik permukaan (Ground/Wall/Roof/Closure) | – | – | M | M | 0.3 | — | U V | T | belum |
| 74 | Klasifikasi semantik otomatis dari normal | – | – | M | S | 0.3 | — | U P | T | belum |
| 75 | Validasi live: watertight, planaritas, orientasi | P | P | M | L | 0.3 | robust | U P V | T | belum |
| 76 | Jendela/pintu sebagai komponen berlubang (opening) | P | – | M | L | 0.4 | Manifold | U P V | T+R | belum |
| 77 | Template fasad (grid jendela per lantai) | – | – | M | L | 0.4 | — | U V | T | belum |
| 78 | Balkon, kolom, ornamen | P | – | S | L | 0.4 | — | U V | T | belum |
| 79 | Library komponen LOD3 | P | P | S | M | 0.4 | — | U | T | belum |
| 80 | Multi-LOD per bangunan | – | – | M | L | 0.4 | — | U V | T | belum |
| 81 | Generalisasi LOD3→LOD2→LOD1 | – | – | S | XL | 0.4 | — | U P V | T | belum |
| 82 | Atribut bangunan (CityGML: fungsi, tahun, tinggi) | – | – | M | S | 0.2 | — | U V | T | belum |

## I/O

| # | Fitur | SU | Hew | Prio | Kompl | Rilis | Library | Test | Jenis | Status |
|---|---|---|---|---|---|---|---|---|---|---|
| 83 | Save/load format native berversi | ✓ | ✓ | M | M | 0.1 | serde, zip | U P E | T+R | parsial |
| 84 | Ekspor CityJSON 2.0 | – | – | M | M | 0.2 | serde, cjval | U V | T+R | belum |
| 85 | Impor CityJSON + round-trip tanpa kehilangan | – | – | M | M | 0.2 | serde_json (+ cjseq untuk CityJSONSeq) | U V | T+R | belum |
| 86 | Ekspor CityGML 2.0 | P | – | M | M | 0.3 | citygml-tools (konversi; Java 17+) | U V | R | belum |
| 87 | Ekspor CityGML 3.0 | – | – | C | M | 0.3 | citygml-tools | U V | R | belum |
| 88 | Ekspor 3D Tiles 1.1 | – | – | S | M | 0.3 | tyler | U | R | belum |
| 89 | glTF/GLB impor-ekspor | P | ✓ | M | S | 0.1 | gltf crate | U | R | parsial |
| 90 | OBJ/STL ekspor | ✓ | P (STL saja) | S | S | 0.1 | — | U | R | parsial |
| 91 | DXF impor (footprint dari CAD) | ✓ | – | S | M | 0.2 | dxf crate | U | R | belum |
| 92 | IFC ekspor dasar | ✓ | – | C | L | 0.5 | IfcOpenShell (Python) | U V | R | belum |
| 93 | USD/USDZ ekspor | P | ✓ | W | M | — | — | — | R | belum |

## Scripting dan otomasi

| # | Fitur | SU | Hew | Prio | Kompl | Rilis | Library | Test | Jenis | Status |
|---|---|---|---|---|---|---|---|---|---|---|
| 94 | Mode headless/CLI (batch) | – | ✓ | S | M | 0.2 | clap | U E | T+R | belum |
| 95 | API Python (proses terpisah) | ✓ (Ruby) | – | S | L | 0.5 | PyO3/JSON-RPC | U E | T+R | belum |
| 96 | Segmentasi atap/fasad ML lokal | – | – | C | XL | 0.5+ | ort (ONNX) | U B | R | belum |

## UX umum

| # | Fitur | SU | Hew | Prio | Kompl | Rilis | Library | Test | Jenis | Status |
|---|---|---|---|---|---|---|---|---|---|---|
| 97 | Shortcut yang bisa diubah (preset gaya SketchUp) | ✓ | ✓ | M | S | 0.1 | — | U | T | belum |
| 98 | Tema terang/gelap | P | ? | C | S | 0.5 | — | G | T | belum |
| 99 | Multi-bahasa (Inggris, Indonesia) | ✓ | ? | S | M | 0.5 | fluent crate (asumsi) | U | R | belum |
| 100 | Autosave + crash recovery | ✓ | ✓ | M | M | 0.1 | — | U E | T | parsial |

Total: 100 fitur. Must = 56, Should = 26, Could = 16, Won't = 2. Hitung ulang setiap tabel berubah:

```bash
awk -F'|' '/^\| [0-9]+ \|/{gsub(/ /,"",$6); c[$6]++} END{for(k in c) print k, c[k]}' docs/plan/feature-matrix.md
```
