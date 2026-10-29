# Tahap 3 — Arsitektur Agent-Friendly

Status: draf 2026-10-29. Bergantung ADR-000: pada jalur A, modul yang sudah ada di Hew dipetakan ke crate Hew dan tidak ditulis ulang.

## 1. Prinsip

1. **Modul kecil, satu arah dependensi.** Setiap modul bisa dipahami dari `SPEC.md`-nya tanpa membaca modul lain.
2. **Inti tanpa I/O dan tanpa UI.** `core-geometry`, `scene`, `commands`, `inference`, dan `lod` bisa diuji headless.
3. **Test adalah pagar.** Tidak ada merge tanpa CI 3 OS hijau (AGENTS.md §10).
4. **Double di model, float di GPU.** Konversi hanya terjadi di `render`, relatif ke pusat lokal (ADR-003).
5. **Gagal dengan jelas.** Operasi yang akan menghasilkan topologi invalid mengembalikan error bertipe, tidak diperbaiki diam-diam. Pola ini sama dengan Hew ([ARCHITECTURE.md](https://github.com/hew3d/hew/blob/main/docs/dev/ARCHITECTURE.md)).

## 2. Layer dan dependensi

```
ui ──► tools ──► commands ──► scene ──► core-geometry
 │       │          │           ▲            ▲
 │       └──► inference ────────┘────────────┘
 ├──► render ──► scene, core-geometry
 ├──► io ──► scene, lod, core-geometry
 └──► plugin-host ──► commands (via API publik saja)
lod ──► scene, core-geometry, commands
```

Aturan: panah hanya ke kanan/bawah. Siklus = CI gagal (cek dengan `cargo deny`/`cargo-modules`; alat final dipilih saat scaffold).

| Modul | Tanggung jawab | Bukan tanggung jawab | Jalur A: padanan Hew |
|---|---|---|---|
| `core-geometry` | Half-edge, face poligon planar, predikat `robust`, toleransi terpusat, validator invariant, adaptor Manifold dan i_overlay | Scene, undo, rendering | `kernel` (mesh), `mesh-heal` |
| `scene` | Pohon dokumen: Groups, Components, Tags, Scenes, atribut, CRS + origin proyek | Operasi geometri | `kernel` (document) |
| `commands` | Command pattern: setiap mutasi = command dengan `apply`/`revert`; riwayat undo; fingerprint pasca-operasi | UI, input | `kernel` (undo) |
| `inference` | Ray picking (BVH), kandidat snap, prioritas, kunci sumbu, inference paralel/tegak lurus | Menggambar | `inference` |
| `tools` | State machine per tool (Line, Rect, Push/Pull, …), menerjemahkan input + inference menjadi command | Rendering | sebagian di UI TypeScript |
| `render` | Buffer GPU, RTC, edge CAD, shadow, section, culling | Logika model | `tessellate` + three.js |
| `ui` | Jendela, toolbar, VCB, outliner, shortcut, i18n | Logika model | shell Tauri + React |
| `lod` | LOD1 extrude, template atap, roof fitting, template fasad, semantik, generalisasi, multi-LOD | Format file | **baru** |
| `io` | Native, CityJSON, CityGML (via citygml-tools), 3D Tiles (via tyler), glTF, OBJ, DXF, GeoJSON, Shapefile, GeoTIFF | Validasi semantik (milik `lod`) | `mesh-export` + importer; **baru** untuk geo |
| `plugin-host` | Proses Python terpisah, API JSON-RPC berversi, izin | Akses langsung ke struktur internal | `api` (rencana Hew) |

Setiap modul memiliki `SPEC.md`: tujuan, API publik, invariant, non-goals, toleransi yang dipakai, dan daftar test. Mengubah API publik berarti mengubah `SPEC.md` di PR yang sama (AGENTS.md §4).

## 3. Model data inti

- **Koordinat**: `f64` meter dalam sistem lokal proyek. Proyek menyimpan `crs` (EPSG) dan `origin` (`f64` ×3) yang memetakan sistem lokal ke CRS. Semua impor ditransformasikan ke lokal; ekspor ditransformasikan balik.
- **Face**: poligon planar dengan lubang (outer loop + inner loops) di atas half-edge. Planaritas dicek dengan toleransi terpusat.
- **Objek bangunan**: `Building { id, attrs, lods: Map<Lod, Geometry> }`. Setiap face pada geometri LOD membawa label semantik opsional (Ground/Wall/Roof/Closure/Window/Door).
- **Toleransi**: satu modul `core-geometry::tol` berisi konstanta bernama (mis. `LENGTH_EPS = 1e-6 m`, `PLANARITY_EPS`, `ANGLE_EPS`), terdokumentasi, dan hanya diubah dengan persetujuan manusia (AGENTS.md §14). Nilai final diputuskan setelah spike presisi.

## 4. Format file native (ADR-006)

```
proyek.csk  (zip)
├── manifest.json     { "format": "csk", "version": 1, "crs": "EPSG:32749", "origin": [x,y,z], ... }
├── scene.json        pohon objek, tags, scenes, styles, atribut
├── geometry/*.bin    buffer half-edge per objek (little-endian, terdokumentasi)
└── materials/        tekstur
```

- Berversi dengan migrasi eksplisit `v(n) → v(n+1)` dan test fixture untuk setiap versi lama.
- Output byte-stable (urutan kunci deterministik) agar bisa di-diff dan diuji dengan hash.
- Validator yang sama dijalankan saat load dan saat edit.
- Pada jalur A, ini adalah perluasan `.hew`, bukan format baru.

## 5. Strategi performa

| Teknik | Dipakai di | Kapan |
|---|---|---|
| BVH per objek + BVH tingkat scene | inference, picking | F0 |
| Instancing Components | render | 0.1 |
| Frustum culling per objek/tile | render | 0.2 |
| LOD rendering (bangunan jauh pakai LOD1) | render + lod | 0.2 |
| Tessellation dan validasi paralel (rayon) | core-geometry, lod | 0.2 |
| Batch LOD di thread worker, UI tidak menunggu | lod, ui | 0.2 |
| Streaming tile untuk scene kota besar | io, render | 0.4 |

Benchmark otomatis (`criterion`) untuk: push/pull pada 10k face, picking pada 1M face, LOD1 batch 10k bangunan, load/save 1M face. Regresi > ambang (AGENTS.md §10, diisi setelah baseline) = CI gagal.

## 6. Presisi numerik

1. Model: `f64`. Render: `f32` relatif ke pusat lokal per objek/tile (RTC).
2. Predikat orientasi/in-circle: `robust` (exact adaptif). Tidak ada `==` pada float; tidak ada epsilon ad hoc (AGENTS.md §6).
3. Setiap test geometri dijalankan dua kali: di origin dan di offset UTM (X=430.000, Y=9.150.000). Hasil harus identik dalam toleransi.
4. Transformasi CRS hanya di batas impor/ekspor (`io`), lewat PROJ.

## 7. Pengujian

| Jenis | Alat | Cakupan | Wajib di CI |
|---|---|---|---|
| Unit | `cargo test` | Semua modul | ✓ 3 OS |
| Property-based / fuzz | `proptest`; `cargo-fuzz` (malam hari) | Setiap operasi yang mengubah mesh: input acak tidak boleh merusak invariant | ✓ (proptest), fuzz terjadwal |
| Replay deterministik | Rekaman sesi → hash scene | Tools + commands | ✓ |
| Golden-image | Render headless (wgpu offscreen / softrender) → bandingkan dengan toleransi piksel | render | ✓ macOS/Linux; Windows bila GPU CI tersedia |
| Validasi ekspor | `val3dity`, `cjval` sebagai proses | io + lod | ✓ |
| Benchmark | `criterion` | Jalur panas | ✓ (laporan + ambang) |
| Model besar | Dataset 3D BAG tile + data sintetis 1M/10M face | render, io | Terjadwal mingguan |

Data uji: tile 3D BAG (CityJSON, NL) sebagai referensi publik; data Indonesia hanya bila lisensinya jelas.
