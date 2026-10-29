# Riset Tahap 0.C + 0.D — Teknologi, Lisensi, Distribusi

Status: draf Sesi 1 (2026-10-28). Semua tautan diakses 2026-10-28. Klaim tanpa tautan ditandai **asumsi**.

## 0. Temuan yang mengubah arah: Hew sudah ada

Hew adalah modeler 3D open source yang secara eksplisit meniru alur kerja SketchUp: menggambar di atas face, push/pull, inference snapping, bahkan key binding yang sama ([README](https://github.com/hew3d/hew), [hew3d.com](https://hew3d.com/)).

| Aspek | Fakta | Sumber |
|---|---|---|
| Kernel | Rust murni, dikompilasi ke WebAssembly; tiap Object memiliki "pulau" mesh **half-edge** dan status watertight | [ARCHITECTURE.md](https://github.com/hew3d/hew/blob/main/docs/dev/ARCHITECTURE.md) |
| Crate | `kernel` (mesh, dokumen, boolean, undo, serialisasi), `inference`, `tessellate`, `mesh-heal`, `mesh-export`, importer per format, `hlr`, `pdfwrite`, `api`, `hew-cli`, `wasm-api` | idem |
| UI / render | TypeScript/React; shell Tauri (desktop) dan web; render three.js di WebGL2; `softrender` untuk snapshot headless | idem |
| Robustness | Operasi yang menghasilkan topologi invalid gagal dengan error bertipe (tidak diperbaiki diam-diam). Debug build memvalidasi setiap mutasi. Panjang `f64` meter; toleransi berupa konstanta bernama. Tidak disebut memakai exact predicates. | idem |
| Undo | Operasi invers direkam; replay harus cocok dengan fingerprint geometri | idem |
| Uji | Property test di kernel, replay sesi deterministik terhadap hash, sedikit E2E | idem |
| Fitur terkirim | Line/Rect/Circle/Arc, push/pull, Follow Me, Offset, Move/Rotate/Scale + array, tape measure, dimensi, teks 3D, group/component, tags, scenes, material, PDF/print, I/O `.skp`/glTF/STL/3MF/USDZ/SVG | [ROADMAP.md](https://github.com/hew3d/hew/blob/main/docs/ROADMAP.md) |
| Belum ada | Geospasial, CRS, koordinat besar, CityJSON/CityGML/IFC, Python, shadows, section plane penuh; plugin API sandboxed masih "longer-term"; WebGPU renderer "maybe" | idem |
| Lisensi | **AGPL-3.0-only** + "Hew Plugin API Exception" (plugin closed-source boleh bila hanya memakai Plugin API) | [README](https://github.com/hew3d/hew) |
| Umur & aktivitas | Repo dibuat 2026-07-14; push terakhir 2026-10-27; rilis v1.3.1 pada 2026-10-28 (5 rilis dalam 11 hari); 128 bintang, 20 fork | GitHub API `repos/hew3d/hew` |
| Bus factor | Kontributor manusia yang tercatat: 1 (`Krenuds`), sisanya bot | GitHub API `contributors` |
| Agen AI | Repo memiliki `CLAUDE.md` dan `docs/agents/` | [README](https://github.com/hew3d/hew) |

Implikasi:
- Sebagian besar Fase 0 dan v0.1 rencana awal (half-edge, push/pull, inference, undo, save/load) sudah ada di Hew, ditulis dengan bahasa (Rust) dan disiplin yang sama dengan yang kita inginkan.
- Ruang yang **belum diisi siapa pun**: alur kerja geospasial dan LOD1–LOD3 (CRS, koordinat UTM, footprint batch, roof fitting, semantik CityGML, val3dity, CityJSON/3D Tiles). Ini persis USP proyek kita.
- Risiko Hew: proyek berumur 3,5 bulan, satu pengelola, lisensi AGPL menular ke turunan, UI berbasis web (three.js/WebGL2) belum terbukti untuk 10M face.
- Risiko hukum yang perlu dicatat: Hew membaca `.skp` lewat reader clean-room "OpenSKP". Kita tidak perlu fitur ini untuk USP; jangan bergantung padanya sebelum risikonya dipahami.

Opsi strategi (keputusan manusia, lihat akhir dokumen):
- **A. Bangun di atas Hew** — fork atau kontribusi upstream; LOD/geospasial sebagai crate tambahan. Lisensi proyek ikut AGPL-3.0.
- **B. Mulai dari nol** — sesuai rencana awal; Hew menjadi referensi UX dan arsitektur saja (tanpa menyalin kode jika lisensi kita bukan AGPL).
- **C. Spike evaluasi 1–2 minggu dulu** (rekomendasi) — build Hew lokal, ukur FPS pada 1M face, uji koordinat UTM besar, baca kernel dan test, tanya pengelola soal penerimaan fitur geospasial. Lalu pilih A atau B dengan data.

Tiga bagian berikut tetap berlaku untuk kedua jalur. Pada jalur A, banyak pilihan (bahasa, UI, render) sudah ditentukan Hew.

## 1. Bahasa inti: Rust vs C++

| Kriteria | Rust | C++ |
|---|---|---|
| Ekosistem geometri | Binding Manifold (`manifold-csg`, Apache-2.0/MIT, butuh CMake + compiler C++) [docs.rs](https://docs.rs/crate/manifold-csg/latest/source/README.md); Clipper2 (`clipper2`, Apache/MIT; C++ asli BSL-1.0) [docs.rs](https://docs.rs/crate/clipper2/0.6.0); `i_overlay` pure Rust [docs.rs](https://docs.rs/crate/i_overlay/8.0.0); predicates Shewchuk `robust` (georust) [docs.rs](https://docs.rs/robust) | Akses langsung CGAL, OpenCascade, IfcOpenShell, OpenUSD, Manifold, Clipper2 |
| Kernel B-rep native | Fornjot dihentikan ("goals were never reached") [GitHub](https://github.com/hannobraun/fornjot) → tidak ada kernel B-rep Rust matang | OpenCascade (matang) |
| Geospasial | `proj` dengan fitur `bundled_proj` [docs.rs](https://docs.rs/crate/proj/0.20.3); `gdal` butuh GDAL terpasang, jalur bundled terbatas [PR #517](https://www.github.com/georust/gdal/pull/517); CityJSON native: `cjval` (validator resmi), `cjseq` [cjval](https://github.com/cityjson/cjval), [cjseq](https://github.com/cityjson/cjseq); 3D Tiles: `tyler` (CityJSON → 3D Tiles 1.1) [docs.rs](https://docs.rs/crate/tyler/0.4.1/source/README.md) | GDAL/PROJ/PDAL native C/C++ |
| Plugin Python | PyO3 mendukung embedding interpreter [docs.rs](https://docs.rs/crate/pyo3/latest) | pybind11 (**asumsi**: setara matang) |
| ML lokal | `ort` (ONNX Runtime, CoreML EP di Apple) [docs.rs](https://docs.rs/crate/ort/latest/source/src/ep/mod.rs) | ONNX Runtime C++ |
| Kode agen | Studi 86.726 error kode LLM: 16,7% error kompilasi Rust adalah pelanggaran ownership/lifetime yang di C/C++ menjadi bug memori diam [ringkasan blog](https://codex.danielvaughan.com/2026/08/13/unreliable-in-practice-llm-generated-code-errors-codex-cli-posttooluse-verification-memory-safety-input-validation/) — angka belum diverifikasi ke paper aslinya. Memory issue mendominasi kerentanan pada kode C/C++ hasil LLM [alphaxiv](https://alphaxiv.org/overview/2502.01853v1) | — |
| Build di Mac | cargo sudah terpasang | CMake belum terpasang (mudah dipasang) |

**Rekomendasi ADR-001: Rust.** Alasan: (1) kompiler menangkap kelas bug memori yang paling sulit direview satu manusia; (2) library wajib §5 yang kita butuhkan untuk LOD tersedia sebagai crate atau binding (Manifold, Clipper2, PROJ, CityJSON, ONNX); (3) jalur A (Hew) juga Rust. Yang ditolak: C++ (ekosistem lebih kaya, tetapi risiko memori pada kode agen dan biaya review lebih tinggi). Mitigasi kelemahan Rust: IfcOpenShell, GDAL penuh, dan PDAL dipanggil lewat proses/plugin Python, bukan FFI langsung (**asumsi** desain, diuji di spike).

## 2. UI

| Opsi | Lisensi | Catatan |
|---|---|---|
| egui | MIT/Apache (**asumsi**, cek repo) | Terbukti untuk aplikasi profesional (Rerun Viewer) [egui README](https://docs.rs/crate/egui/0.34.2/source/README.md); aksesibilitas via AccessKit (opsional; status macOS/Linux perlu dicek) [AccessKit](https://github.com/AccessKit/accesskit) |
| Slint | GPLv3 / Royalty-free / komersial | GPLv3 bebas untuk proyek open source [LICENSE](https://cdn.jsdelivr.net/gh/slint-ui/slint@1.16.0/LICENSE.md) |
| Qt/QML (via CXX-Qt) | LGPLv3/GPLv3; beberapa modul GPL-only | Kewajiban LGPL: pengguna harus bisa menautkan ulang Qt [Qt FAQ](https://www.qt.io/development/open-source-lgpl-obligations); CXX-Qt aktif (v0.8.1, Feb 2026) tetapi tidak membungkus QWidgets [GitHub](https://github.com/kdab/cxx-qt) |
| Tauri + web | MIT/Apache (**asumsi**) | Jalur Hew; satu UI untuk desktop + browser |
| Dear ImGui | MIT (**asumsi**) | Cocok alat internal; kurang cocok UI pengguna akhir (**asumsi**) |

**Rekomendasi sementara ADR-002**: jalur B → egui (paling sedikit lapisan, satu bahasa, terbukti di Rerun); jalur A → ikut Tauri + React milik Hew. Keputusan final setelah spike.

## 3. Rendering

- wgpu: satu API untuk Metal (macOS), Vulkan, DX12; lisensi MIT/Apache-2.0 [gfx-rs blog](https://gfx-rs.github.io/2021/07/16/release-0.9-future.html).
- Presisi koordinat besar: teknik **RTC (relative-to-center)** — posisi disimpan relatif terhadap pusat, matriks model-view menempatkan pusat relatif ke kamera, sehingga float32 GPU tidak mengurangkan angka besar ([CESIUM_RTC](https://github.com/liwcv/glTF/blob/master/extensions/1.0/Vendor/CESIUM_RTC/README.md), [AGI "Precisions, Precisions"](https://help.agi.com/AGIComponents/html/BlogPrecisionsPrecisions.htm)). Satu pusat tidak cukup untuk geometri sangat luas; gunakan pusat per klaster (AGI menyarankan simpul tetap dalam ~131 km dari pusat klasternya). Untuk proyek kita: simpan double di model, render float32 relatif ke origin lokal per tile/bangunan.
- Edge rendering gaya CAD, shadow map, PBR: belum diriset (Sesi 2/3).

**Rekomendasi ADR-003**: wgpu (jalur B). Jalur A: three.js milik Hew; WebGPU masih "maybe" di roadmap Hew.

## 4. Geometri dan inference

- Half-edge ditulis sendiri (sesuai AGENTS.md §5) — atau dipakai dari Hew (jalur A).
- Boolean 3D: Manifold, dengan tujuan utama "guaranteed manifold output" [GitHub](https://github.com/elalish/manifold). Catatan: Manifold bekerja pada mesh segitiga; face poligon planar gaya SketchUp perlu dipetakan bolak-balik (**asumsi** risiko, diuji di spike).
- 2D (footprint, offset): Clipper2 atau `i_overlay`.
- Predicates: `robust` (port langsung `predicates.c` Shewchuk) [docs.rs](https://docs.rs/robust).
- CGAL: paket fondasi LGPL, paket algoritma tingkat tinggi GPL [cgal.org](https://www.cgal.org/license.html) — boleh dipakai bila lisensi proyek GPL.
- OpenCascade: LGPL-2.1 + Open CASCADE Exception [OCCT docs](https://dev.opencascade.org/doc/overview/html/occt_public_license.html) — hanya bila kelak butuh STEP/B-rep.
- Perilaku inference SketchUp (referensi UX, bukan kode): titik/garis/face, warna sumbu merah/hijau/biru/magenta, Shift mengunci inference, tombol panah mengunci sumbu [SketchUp Help](https://help.sketchup.com/en/sketchup/introducing-drawing-basics-and-concepts).

## 5. Format dan validasi

| Kebutuhan | Pilihan | Lisensi / catatan |
|---|---|---|
| Validasi geometri LOD | val3dity (CLI, `brew install val3dity`) | GPLv3; hati-hati bila ditautkan sebagai library [docs](https://val3dity.readthedocs.io/main/usage/), [GitHub](https://github.com/tudelft3d/val3dity). Rencana: dipanggil sebagai proses di CI, tidak ditautkan. |
| Validasi skema CityJSON | `cjval` (Rust) | [GitHub](https://github.com/cityjson/cjval) |
| CityJSON I/O | `serde_json` + pemetaan scene sendiri; `cjseq` (MIT) hanya untuk konversi/stream CityJSONSeq | [GitHub](https://github.com/cityjson/cjseq) |
| CityGML 2/3 | citygml-tools (konversi dari/ke CityJSON) | Apache-2.0, aplikasi **Java 17+** ([GitHub](https://github.com/citygml4j/citygml-tools)). Butuh JRE di CI dan di mesin pengguna; risiko packaging (R8). |
| 3D Tiles | `tyler` (CityJSON → 3D Tiles 1.1) | Apache-2.0 ([GitHub](https://github.com/3DGI/tyler)) |
| IFC | IfcOpenShell (LGPL-3.0-or-later; add-on Blender GPL) | [PyPI](https://pypi.org/project/ifcopenshell) — dipakai lewat Python |
| CRS | PROJ (`proj` crate, `bundled_proj`) | [docs.rs](https://docs.rs/crate/proj/0.20.3) |
| Raster/vektor | GDAL (`gdal` crate atau Python) | Windows butuh GDAL terpasang [gdal-sys](https://docs.rs/crate/gdal-sys/0.4.0/source/Readme.md) |

## 6. Plugin API

- Python via PyO3 embedding [docs.rs](https://docs.rs/crate/pyo3/latest); distribusi runtime Python dalam paket masih perlu riset (PyOxidizer [docs](https://pyoxidizer.readthedocs.io/en/stable/pyoxidizer_rust_generic_embedding.html)).
- Alternatif yang lebih aman: plugin sebagai proses terpisah dengan API terdokumentasi (pendekatan yang direncanakan Hew).
- Kompatibilitas Ruby API SketchUp: Google v. Oracle (2021) menyatakan reimplementasi API Java oleh Google adalah fair use, tetapi putusannya sempit dan bergantung fakta; Mahkamah tidak memutuskan apakah API dapat dilindungi hak cipta [LWN](https://lwn.net/Articles/851617), [Wiley](https://www.wiley.law/alert-Supreme-Court-Determines-Copying-of-Software-API-Can-Constitute-Fair-Use). Hukum di luar AS bisa berbeda. **Rekomendasi: jangan meniru Ruby API.** Manfaatnya kecil untuk USP LOD, risikonya tidak sebanding. (Ini bukan nasihat hukum.)

## 7. Lisensi proyek (0.D)

- Dependensi kandidat yang membatasi pilihan: CGAL (paket GPL), Slint (GPLv3), val3dity (GPLv3, tetapi dipanggil sebagai proses), Hew (AGPL-3.0-only, bila jalur A).
- Dependensi permisif: wgpu, Manifold, Clipper2 (BSL-1.0), PROJ, GDAL, PyO3, `ort` (**asumsi** sebagian; diverifikasi saat ADR).
- **Rekomendasi ADR-005**: jalur B → **GPL-3.0-or-later** (kompatibel dengan CGAL GPL, Slint GPLv3, Qt LGPL; tetap memenuhi syarat program signing gratis yang menuntut lisensi OSI). Jalur A → **AGPL-3.0-only** (wajib mengikuti Hew). GPLv3 §13 mengizinkan penggabungan dengan karya AGPLv3 [GPLv3](https://www.gnu.org/licenses/gpl-3.0.html#section13).
- Ditolak: MIT/Apache (menutup akses ke CGAL GPL dan Slint GPL tanpa manfaat nyata untuk proyek satu orang tanpa model bisnis); MPL-2.0 (alasan serupa).

## 8. Distribusi tanpa code signing berbayar (0.D)

| OS | Kondisi | Solusi tanpa biaya |
|---|---|---|
| macOS | Sejak Sequoia, Control-click tidak lagi melewati Gatekeeper; pengguna harus ke System Settings → Privacy & Security → **Open Anyway** [AppleInsider](https://appleinsider.com/articles/24/08/06/apple-removes-control-click-option-for-skipping-gatekeeper-in-macos-sequoia), [MacRumors](https://www.macrumors.com/2024/08/06/macos-sequoia-gatekeeper-security-change/). Developer ID + notarisasi butuh Apple Developer Program 99 USD/tahun; keringanan biaya hanya untuk nonprofit/sekolah/pemerintah [Belkadan](https://belkadan.com/developer-id-for-freeware), [Apple Forums](https://developer.apple.com/forums/thread/121113). | Panduan instalasi bergambar; build dari source (`cargo install`); **tap Homebrew sendiri** — tap resmi `homebrew-cask` menonaktifkan cask yang gagal pemeriksaan Gatekeeper mulai September 2026, dan `--no-quarantine` didepresiasi [Homebrew 5.0.0](https://brew.sh/2025/11/12/homebrew-5.0.0/). |
| Windows | Binary tanpa tanda tangan memicu SmartScreen | **SignPath Foundation**: sertifikat gratis untuk proyek dengan lisensi OSI tanpa dual-licensing; publisher tercatat sebagai SignPath Foundation [signpath.org](https://signpath.org/), [about.signpath.io](https://about.signpath.io/product/open-source). Contoh pengguna: Pandoc [kebijakan](https://pandoc.org/code-signing-policy.html). |
| Linux | Tidak ada gatekeeper setara | Flatpak/Flathub dan AppImage (**asumsi**: gratis; verifikasi saat packaging) |

Dampak ke keputusan: lisensi tunggal OSI (tanpa dual-license) agar memenuhi syarat SignPath.

## 9. Pertanyaan terbuka (untuk Sesi 2–3)

1. Jalur A/B/C (Hew)? — menentukan ADR-001..003 dan ADR-005.
2. Apakah three.js/WebGL2 (Hew) sanggup 1M/10M face di M4 16 GB? Ukur di spike.
3. Pemetaan face poligon planar ↔ mesh segitiga Manifold: biaya dan risiko presisi.
4. Lisensi `tyler`, citygml-tools, egui, Tauri: verifikasi di repo masing-masing.
5. Status AccessKit di macOS/Linux untuk egui.
6. Cara mengemas runtime Python dalam aplikasi tanpa signing.
