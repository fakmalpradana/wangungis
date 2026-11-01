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
- Toleransi: konstanta Hew (`POINT_MERGE` = `PLANE_DIST` = 1e-9 m) dipertahankan (K6, 2026-11-01). Syaratnya, koordinat lokal kernel ≤ **±50 km** dari origin proyek. Batas ini cukup untuk satu kota: DKI Jakarta daratan berukuran ±45 km melintang (**asumsi**, tanpa Kepulauan Seribu).
  - Sweep S2 tidak menolak satu operasi pun sampai ±2,1e6 m, tetapi hanya untuk kotak yang diputar pada sumbu z. Bidang miring (atap), lingkaran, dan boolean belum diuji pada jarak itu. Jadi ±1.000 km tidak dipakai.
  - Di ±50 km, ulp `f64` = 2⁻³⁷ ≈ 7,3e-12 m, yaitu < 1% toleransi.
  - Syarat F0: property test push/pull dan boolean dengan bidang miring dan lingkaran pada ±50 km harus lulus sebelum batas ini dikunci.
  - Proyek yang lebih luas memakai lebih dari satu origin. Ini di luar scope v0.1–v0.2.
  - Mengubah toleransi butuh persetujuan manusia (AGENTS.md §14).
