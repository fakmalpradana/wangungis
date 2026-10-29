# AGENTS.md — Aturan Bersama Semua Agen (Claude Code, Gemini)

Sumber kebenaran tunggal. CLAUDE.md dan GEMINI.md hanya mengimpor file ini.
Bagian bertanda `TODO` diisi setelah riset (Tahap 0) dan ADR selesai. Jangan menebak nilainya.

## 1. Proyek
- Nama: **WangunGIS** (identifier: `wangungis`; folder repo lokal masih `citySketch`). Open source, lisensi: `TODO` (AGPL-3.0-only diterima, final di ADR-005 setelah spike).
- Tujuan: aplikasi pemodelan 3D desktop dengan UX dan fitur setara SketchUp, fokus awal pemodelan bangunan LOD1–LOD3 yang cepat dan presisi.
- Platform: macOS, Windows, Linux. Satu basis kode. Dev utama di macOS (Apple Silicon).
- Pelaksana: 1 manusia (pemilik/reviewer) + agen AI. Tidak ada tim lain, tidak ada budget uang.
- Clean-room: tiru alur kerja dan perilaku UX SketchUp, JANGAN salin kode, aset, ikon, nama, atau merek dagangnya.

## 2. Prinsip (urutan prioritas)
1. Benar dan presisi sebelum cepat. Geometri yang salah merusak semuanya.
2. Reuse library matang sebelum menulis sendiri (lihat §5).
3. Sederhana, kecil, mudah dipahami manusia. Satu manusia harus sanggup mereview semuanya.
4. Setiap perilaku punya test otomatis. Tanpa test, fitur dianggap belum ada.
5. Jangan mengarang. Jika tidak tahu, baca kode/dokumen, cari sumber, atau tanya manusia.

## 3. Stack
- Bahasa inti: `TODO (Rust atau C++, putusan ADR-001)`
- UI: `TODO (ADR-002)`
- Rendering: `TODO (ADR-003)`
- Kernel geometri: `TODO (ADR-004)`
- Scripting/plugin: Python (utama), opsi WASM `TODO`
- Build/CI: GitHub Actions (macOS, Windows, Linux), semua wajib hijau.

## 4. Struktur repo (target)
```
core-geometry/   half-edge mesh, operasi geometri, robust predicates
scene/           scene graph, Groups, Components, Tags, Scenes
commands/        command pattern, undo/redo
inference/       inference engine (snapping, axis lock, ray picking)
tools/           tool interaksi (Line, Rect, Push/Pull, Follow Me, ...)
render/          renderer, edge rendering, shadow
ui/              antarmuka, VCB, toolbar, shortcut
lod/             LOD1–3: footprint extrude, roof fitting, template fasad, semantik
io/              CityJSON, CityGML, 3D Tiles, IFC, glTF/OBJ, DXF, GeoJSON/Shapefile/GeoTIFF
plugin-host/     API Python
docs/            adr/, spec/, user/
tests/           unit, property-based, golden-image, benchmark
```
Setiap modul punya `SPEC.md` (tujuan, API publik, invariant, non-goals). Ubah kode publik = ubah SPEC.md di PR yang sama.

## 5. Reuse vs tulis sendiri
Wajib pakai library matang (jangan tulis ulang):
- Kernel/booleans/mesh ops: pilihan hasil ADR (CGAL / Manifold / OpenCascade / libigl / Clipper2)
- CRS: PROJ. Raster/vektor: GDAL. Point cloud: PDAL. IFC: IfcOpenShell.
- I/O mesh: assimp / glTF / OpenUSD. Validasi LOD: val3dity. CityJSON: cjio / citygml-tools sebagai referensi dan validator.

Memang harus ditulis sendiri (inti diferensiasi):
- Inference engine, tool interaksi gaya SketchUp, scene model Groups/Components, operasi Push/Pull dan Intersect di atas half-edge, LOD engine (roof fitting, template fasad).

Menambah dependensi baru: tulis ADR singkat (lisensi kompatibel, ukuran, aktivitas proyek, alternatif). Dependensi proprietary/berbayar dilarang.

## 6. Aturan geometri
- Koordinat world: double precision. Gunakan floating origin / origin shifting untuk koordinat UTM besar.
- Predikat geometri kritis (orientasi, in-circle, bidang) memakai robust/exact predicates. Dilarang membandingkan float dengan `==` atau epsilon ad hoc. Toleransi dipusatkan di satu modul dan didokumentasikan.
- Setiap operasi mengubah mesh wajib menjaga invariant: manifold, orientasi normal konsisten, tidak ada face degenerate. Tambahkan assert/validator dan test.
- Operasi baru wajib punya property-based/fuzz test (input acak, pasti tidak merusak invariant).
- Export LOD wajib lolos val3dity (uji otomatis di CI).

## 7. PROTOKOL SKILL (WAJIB)
Skill `caveman` dan `ponytail` terpasang di Claude Code. Agen lain pakai padanan (`.gemini/skills` atau aturan setara); jika tidak setara, catat di log sesi.
Terverifikasi 2026-10-28: caveman 4.0.1 (mode: caveman, ultracave, megacave) dan ponytail 4.7.0 (level: lite, full, ultra). Gemini memakai padanan berbentuk aturan di `GEMINI.md`.

Awal setiap sesi (maks 10 baris):
1. Daftar skill/plugin aktif. Baca SKILL.md `caveman` dan `ponytail` (dan turunannya: review/audit/debt/gain bila ada).
2. Ringkas 3 baris per skill: fungsi, mode, perintah. Jangan mengarang. Tidak ditemukan = katakan dan catat.
3. Pastikan mode yang diinginkan benar-benar aktif; jangan percaya perintah ganti mode tanpa verifikasi.

Pembagian:
| Skill | Berlaku | TIDAK berlaku |
|---|---|---|
| caveman (output ringkas) | status, log sesi, ringkasan antar-sesi, komentar review, laporan debug | SPEC.md, ADR, README, dokumen pengguna, laporan riset, pesan error untuk pengguna, kode dan identifier, peringatan keamanan/aksi destruktif/ambigu (pakai bahasa normal penuh) |
| ponytail (kode minimal, reuse dulu) | semua implementasi, refactor, review | mengurangi test, validasi input, error handling, robust predicates, toleransi numerik, sandbox plugin, aksesibilitas UI |

Tangga ponytail sebelum menulis kode: perlu ada? sudah ada di stdlib/dependensi/library terpilih? ada fitur platform? baru tulis versi paling kecil yang benar.
Mode usulan (verifikasi setelah baca SKILL.md): ringan untuk `core-geometry`/`inference`, standar untuk fitur umum, agresif untuk boilerplate, wrapper I/O, glue UI.
Ukur dampak, jangan percaya klaim: catat baris kode (`tokei`), jumlah dependensi (`cargo tree`), waktu build CI, dan token per sesi per fase. `/ponytail-gain` hanya menampilkan median benchmark plugin, bukan ukuran repo ini. Ekspektasi realistis: `docs/research/skill.md`.

## 8. Alur setiap tugas
1. Baca SPEC.md modul terkait dan test yang ada.
2. Rencana 5 baris (caveman). Tugas > 1 sesi: pecah dulu.
3. Tangga ponytail → implementasi minimal yang benar.
   3a. Cek apakah fitur sudah ada di Hew (jalur A, ADR-000) atau di library terpilih sebelum menulis.
4. Tulis/ubah test dulu atau bersamaan. Jalankan test lokal.
5. `ponytail-review` (bila ada) untuk over-engineering.
6. Review silang agen lain khusus KEBENARAN (invariant, edge case, presisi), bukan hanya ukuran kode.
7. CI 3 OS hijau.
8. Update SPEC.md/ADR bila perlu. Tulis log sesi (§11).

## 9. Perintah (isi setelah scaffold)
- Build: `TODO`
- Test semua: `TODO`
- Test satu modul: `TODO`
- Lint/format: `TODO`
- Benchmark: `TODO`
- Validasi ekspor LOD (val3dity): `TODO`

## 10. Definisi selesai (semua wajib)
- [ ] Test unit + property-based untuk perilaku baru lulus
- [ ] CI macOS/Windows/Linux hijau
- [ ] Tidak ada regresi benchmark > `TODO`%
- [ ] SPEC.md/ADR diperbarui, bahasa normal
- [ ] Checklist skill di bawah lulus
- [ ] Manusia dapat menjelaskan perubahan dari ringkasan PR (jika tidak, sederhanakan)

Checklist skill akhir tugas: skill dibaca/aktif? tangga ponytail dijalankan dan dicatat? test tidak berkurang? dokumen manusia ditulis bahasa normal? Laporkan lulus/gagal.

## 11. Log sesi
Tulis di `docs/sessions/YYYY-MM-DD-<topik>.md` (gaya caveman, maks 15 baris): tujuan, yang selesai, keputusan, yang gagal, langkah berikut. Dipakai sebagai konteks sesi berikutnya, jangan mengandalkan memori percakapan.

## 12. Peran agen
- Claude Code: perancangan, implementasi modul inti (geometry, inference, tools), refactor besar, ADR, SPEC.
- Gemini: implementasi modul I/O dan format, test tambahan, benchmark, dokumentasi, review silang pekerjaan Claude.
- Aturan: agen yang menulis kode tidak jadi satu-satunya reviewer kodenya. Tukar peran per PR.
- Pembagian ini boleh diubah manusia sesuai hasil.

## 13. Git
- Branch: `feat/<modul>-<topik>`, `fix/...`, `refactor/...`, `docs/...`, `spike/...` (spike tidak di-merge ke main; hanya hasil tulisannya)
- Commit kecil, Conventional Commits (`feat(inference): ...`).
- Main selalu hijau. Tidak ada push langsung ke main.
- PR: ringkasan 3–5 baris, risiko, test yang ditambah, hasil benchmark bila relevan.

## 14. Checkpoint manusia
Wajib persetujuan manusia sebelum: menambah dependensi, mengubah format file native, mengubah API publik, mengubah toleransi geometri, menghapus test, atau perubahan lisensi. Review manusia rutin: tiap akhir fase + tiap 2 minggu baca ulang SPEC.md modul inti.

## 15. Dilarang
- Menyalin kode/aset/ikon/merek SketchUp atau software proprietary lain.
- Dependensi proprietary/berbayar.
- Menghapus atau melemahkan test agar CI hijau.
- Mengarang fakta (fitur kompetitor, API library, benchmark). Tanpa sumber = tandai "asumsi".
- Aksi destruktif (hapus file/branch, force push, ubah CI secret) tanpa konfirmasi manusia.

## 16. Anggaran konteks dan token
- Satu sesi = satu tugas kecil dengan konteks terfokus (modul + SPEC + test terkait). Jangan muat seluruh repo.
- Simpan hasil riset ke `docs/research/` supaya tidak diulang.
- Akhir sesi: log sesi + ringkasan 5 baris untuk sesi berikutnya.
