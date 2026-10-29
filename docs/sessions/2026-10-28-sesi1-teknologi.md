# 2026-10-28 Sesi 1 — riset teknologi + lisensi

Tujuan: Tahap 0.C + 0.D. Output: `docs/research/teknologi.md`.

- Temuan utama: Hew (github.com/hew3d/hew) = modeler gaya SketchUp, kernel half-edge Rust, Tauri+React+three.js, AGPL-3.0-only. Repo dibuat 2026-07-14, v1.3.1 rilis 2026-10-28, 1 pengelola manusia. Tidak ada geospasial/LOD/CRS.
- Dampak: Fase 0 + v0.1 rencana awal sudah ada di Hew. USP LOD/geospasial masih kosong.
- Usul: jalur C = spike evaluasi Hew 1–2 minggu, lalu pilih A (bangun di atas Hew, AGPL) atau B (dari nol, GPL-3.0-or-later).
- Rekomendasi sementara: Rust; egui+wgpu (jalur B); Manifold + Clipper2/i_overlay + `robust`; val3dity via proses CLI; RTC untuk koordinat besar; jangan tiru Ruby API.
- Distribusi: macOS tanpa notarisasi = "Open Anyway"; Homebrew resmi menolak cask unsigned mulai Sep 2026, jadi pakai tap sendiri; Windows lewat SignPath Foundation (syarat: lisensi OSI tanpa dual-license).
- Tangga ponytail: rung 1/4 (apakah perlu dibangun? sudah ada?) memicu temuan Hew.
- Belum: lisensi tyler/citygml-tools/egui/Tauri, edge rendering, shadow, AccessKit macOS.

Langkah berikut: manusia memilih jalur A/B/C. Lalu Sesi 2 (kompetitor + pain point; Hew masuk matriks).
