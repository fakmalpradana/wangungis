# ADR-004: Kernel geometri

- Status: Proposed

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
