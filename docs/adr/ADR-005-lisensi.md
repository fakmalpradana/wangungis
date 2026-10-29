# ADR-005: Lisensi proyek

- Status: Proposed (bergantung ADR-000)

## Keputusan
- Jalur A: **AGPL-3.0-only** (wajib mengikuti Hew).
- Jalur B: **GPL-3.0-or-later**.
- Satu lisensi OSI, tanpa dual-licensing, agar memenuhi syarat code signing gratis SignPath Foundation ([signpath.org](https://signpath.org/)).

## Catatan keputusan
- 2026-10-29: manusia menerima AGPL-3.0-only (K1, kriteria #7 ADR-000). Bila spike memilih jalur A, ADR ini menjadi Accepted dengan AGPL-3.0-only.
- Konsekuensi untuk jalur B: karena AGPL sudah diterima, jalur B juga boleh memakai AGPL-3.0-only. Dengan begitu kode Hew boleh disalin dengan atribusi (ADR-000, Konsekuensi). GPL-3.0-or-later tetap default jalur B; pilihan final diambil saat K2.

## Alasan
- Kompatibel dengan dependensi yang mungkin dipakai: CGAL paket GPL ([cgal.org](https://www.cgal.org/license.html)), Slint GPLv3, Qt LGPL, Manifold Apache-2.0, Clipper2 BSL-1.0.
- val3dity (GPLv3) dipanggil sebagai proses CLI, tidak ditautkan.

## Alternatif ditolak
- **MIT/Apache-2.0 / MPL-2.0**: menutup akses ke paket CGAL GPL dan Slint GPL, tanpa manfaat nyata untuk proyek satu orang tanpa model bisnis.

## Catatan
Ini bukan nasihat hukum. Cek ulang lisensi `tyler`, citygml-tools, egui, dan Tauri sebelum ADR ini diterima.
