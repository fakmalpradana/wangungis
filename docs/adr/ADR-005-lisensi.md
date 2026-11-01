# ADR-005: Lisensi proyek

- Status: **Accepted** 2026-11-01: **AGPL-3.0-only** (K1 + K2 = A2, ADR-000). A2 memakai crate Hew yang AGPL-3.0-only, jadi pilihan GPL jalur B tidak berlaku.

## Keputusan
- Jalur A: **AGPL-3.0-only** (wajib mengikuti Hew).
- Jalur B: **GPL-3.0-or-later**.
- Satu lisensi OSI, tanpa dual-licensing, agar memenuhi syarat code signing gratis SignPath Foundation ([signpath.org](https://signpath.org/)).

## Catatan keputusan
- 2026-10-29: manusia menerima AGPL-3.0-only (K1, kriteria #7 ADR-000). Bila spike memilih jalur A, ADR ini menjadi Accepted dengan AGPL-3.0-only.
- Konsekuensi untuk jalur B: karena AGPL sudah diterima, jalur B juga boleh memakai AGPL-3.0-only. Dengan begitu kode Hew boleh disalin dengan atribusi (ADR-000, Konsekuensi). GPL-3.0-or-later tetap default jalur B; pilihan final diambil saat K2.

## Alasan
- Kompatibel dengan dependensi yang mungkin dipakai: CGAL paket GPL ([cgal.org](https://www.cgal.org/license.html)), Slint GPLv3, Qt LGPL, Manifold Apache-2.0, Clipper2 BSL-1.0.
- val3dity (GPLv3) dipanggil sebagai proses CLI, tidak ditautkan. Alasannya isolasi build C++/CGAL, bukan ketidakcocokan lisensi: lisensi GPL-3.0-or-later maupun AGPL-3.0-only sama-sama kompatibel dengan GPLv3.

## Alternatif ditolak
- **MIT/Apache-2.0 / MPL-2.0**: menutup akses ke paket CGAL GPL dan Slint GPL, tanpa manfaat nyata untuk proyek satu orang tanpa model bisnis.

## Catatan
Ini bukan nasihat hukum.

Lisensi yang sudah dicek pada 2026-10-29 (GitHub API):
- `tyler`: Apache-2.0.
- citygml-tools: Apache-2.0.
- `cjseq` dan `cjval`: MIT.
- val3dity: GPL-3.0.
- Hew: AGPL-3.0.

- egui, wgpu, Tauri, `manifold-csg`, manifold3d: GitHub API melaporkan Apache-2.0 (dicek 2026-10-29). Detail dual-license MIT/Apache belum dicek di file LICENSE masing-masing.

**Risiko SignPath.** Syarat SignPath Foundation berbunyi: "OSI-approved Open Source license without commercial dual-licensing for all components" ([signpath.org/terms](https://signpath.org/terms)). Slint (GPLv3/royalty-free/komersial), CGAL, dan Qt juga ditawarkan dengan lisensi komersial. Membundel salah satunya bisa membuat proyek ditolak SignPath. Konfirmasi ke SignPath sebelum memilih library UI atau CGAL. Pilihan yang tidak terkena risiko ini: egui, wgpu, Manifold, Clipper2.

Hew Plugin API Exception adalah izin tambahan menurut AGPL §7. Izin tambahan seperti ini tidak membuat proyek menjadi dual-license. Penilaian ini **asumsi** dan belum dikonfirmasi ke SignPath.
