# ADR-002: UI

- Status: Proposed (bergantung ADR-000)

## Keputusan
- Jalur A: ikuti Hew — Tauri + TypeScript/React.
- Jalur B: **egui** (eframe) di atas wgpu.

## Alasan (jalur B)
- Satu bahasa (Rust), lapisan paling sedikit.
- Terbukti untuk aplikasi profesional: Rerun Viewer ([egui README](https://docs.rs/crate/egui/0.34.2/source/README.md)).
- Aksesibilitas lewat AccessKit (fitur opsional) ([AccessKit](https://github.com/AccessKit/accesskit)); status macOS/Linux wajib dicek di spike.

## Alternatif ditolak
- **Slint**: GPLv3 cocok, tetapi ekosistem widget untuk aplikasi CAD lebih kecil (asumsi; cek ulang bila egui gagal aksesibilitas).
- **Qt/QML via CXX-Qt**: matang, tetapi tidak ada binding QWidgets dan kewajiban relink LGPL menambah kerja packaging ([CXX-Qt](https://github.com/kdab/cxx-qt), [Qt](https://www.qt.io/development/open-source-lgpl-obligations)).
- **Dear ImGui**: cocok untuk alat debug, kurang cocok untuk UI pengguna akhir (asumsi).
- **Tauri + web di jalur B**: dua bahasa dan dua tumpukan build untuk satu orang.

## Bila opsi A2 dipilih (ADR-000, menunggu K2)
Keputusan untuk A2 sama dengan jalur B: **egui** (eframe) di atas wgpu. Lisensi Apache-2.0 menurut GitHub API (2026-10-29). Tool UI Hew (±37 ribu baris TypeScript) menjadi referensi perilaku. Karena AGPL sudah diterima (K1), porting langsung dengan atribusi juga diizinkan. Orkestrasi di `crates/wasm-api` (±24,7 ribu baris Rust) dievaluasi untuk dipakai ulang sebagai library biasa.
