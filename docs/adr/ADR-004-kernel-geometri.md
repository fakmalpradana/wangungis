# ADR-004: Kernel geometri

- Status: **Accepted** 2026-11-01 untuk A2: kernel Hew + bagian "Opsi A2" di bawah (K2, ADR-000). Keputusan umum di atasnya berlaku bila tidak bertentangan dengan bagian A2.

## Keputusan
- Struktur utama: **half-edge** dengan face poligon planar (gaya SketchUp). Jalur A: milik Hew. Jalur B: ditulis sendiri (AGENTS.md §5).
- Predikat: `robust` (port Shewchuk) untuk orientasi/in-circle ([docs.rs](https://docs.rs/robust)).
- Boolean 3D: **Manifold** (`manifold-csg`), dengan tujuan output manifold terjamin ([GitHub](https://github.com/elalish/manifold)).
- Poligon 2D (footprint, offset): **i_overlay** (pure Rust) atau Clipper2.
- Toleransi: konstanta bernama di satu modul (AGENTS.md §6).

## Alternatif ditolak
- **CGAL**: kuat, tetapi C++ dan sebagian GPL; dipertimbangkan ulang hanya untuk operasi yang Manifold tidak punya.
- **OpenCascade**: B-rep NURBS tidak dibutuhkan untuk LOD1–3; dipertimbangkan bila kelak perlu STEP.
- **Fornjot / kernel B-rep Rust**: proyek dihentikan ([GitHub](https://github.com/hannobraun/fornjot)).

## Risiko yang diuji di spike
- Pemetaan face poligon ↔ mesh segitiga Manifold dan kembalinya tanpa kehilangan planaritas.

## Opsi A2 (dipilih, K2 2026-11-01)
- Half-edge, push/pull, undo, dan boolean memakai `kernel` Hew.
- Predikat: Hew baru memakai orientasi 2D eksak. Kebutuhan `robust` 3D dievaluasi ulang.
- Manifold menjadi opsional, untuk boolean berat dan LOD3.
- Toleransi: konstanta Hew (`POINT_MERGE` = `PLANE_DIST` = 1e-9 m) dipertahankan (K6, **dikunci 2026-11-01** lewat `scene/tests/kernel_bound.rs`).
  - **Satu `Document` Hew hanya memuat geometri dalam radius 1,5 km dari origin-nya** (`KERNEL_FRAME_BOUND_M`).
    - Push/pull pada bidang miring dan ekstrusi lingkaran lulus sampai ±50 km.
    - Tetapi boolean Hew pada kotak yang diputar mulai ditolak (`DegenerateContact`) antara 2,5 dan 2,8 km: 20/3.000 kasus di 2,79 km, 47/3.000 di 3 km, dan 63/72 rotasi di 10 km. Sampai 2,5 km tidak ada yang ditolak (0/3.000).
    - Batas 1,5 km memberi margin ±40%.
  - **Akibatnya, proyek dipecah menjadi tile kernel.** Satu `Document` Hew per tile 1 km, dengan frame di pusat tile.
    - Bangunan masuk ke tile titik acuannya. Geometrinya tetap ≤ 1,5 km dari frame, karena pojok tile berjarak 0,71 km dan bangunan < 0,5 km.
    - Posisi tile relatif terhadap origin proyek berupa `f64`, dengan batas ±`PROJECT_BOUND_M` = 50 km per sumbu.
    - Tile kernel sejalan dengan tile render (RTC) dan tile inference.
  - Component Hew tidak bisa menggantikan tile kernel. Mengedit component ("explode session") memindahkan geometri ke koordinat world (`document.rs`, `open_explode_session`), jadi koordinatnya kembali jauh dari origin.
  - Operasi lintas tile (boolean antara dua bangunan di tile berbeda, atau bangunan yang dipindah keluar tile) dijalankan dengan memindahkan operand ke `Document` sementara di sekitar titik tengahnya. Rancangannya dibuat di v0.1.
  - Mengubah toleransi butuh persetujuan manusia (AGENTS.md §14).
