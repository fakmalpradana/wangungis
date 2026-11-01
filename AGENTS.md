# AGENTS.md — Aturan Bersama Semua Agen (Claude Code, Gemini)

Sumber kebenaran tunggal. CLAUDE.md dan GEMINI.md hanya mengimpor file ini.
Bagian bertanda `TODO` diisi setelah riset (Tahap 0) dan ADR selesai. Jangan menebak nilainya.

## 1. Proyek
- Nama: **WangunGIS** (identifier: `wangungis`; folder repo lokal masih `citySketch`). Open source, lisensi: **AGPL-3.0-only** (ADR-005).
- Tujuan: aplikasi pemodelan 3D desktop dengan UX dan fitur setara SketchUp, fokus awal pemodelan bangunan LOD1–LOD3 yang cepat dan presisi.
- Platform: macOS, Windows, Linux. Satu basis kode. Dev utama di macOS (Apple Silicon).
- Pelaksana: 1 manusia (pemilik, pengambil keputusan) + agen AI. Penguji beta: 3D engineer milik pemilik, menguji perilaku aplikasi dan memberi umpan balik langsung. Tidak ada budget uang.
- Strategi: jalur **A2** (ADR-000, K2 2026-11-01). Crate inti Hew (`kernel`, `inference`, `tessellate`, `mesh-export`) dipakai sebagai library, dengan aplikasi native sendiri.
- Clean-room: tiru alur kerja dan perilaku UX SketchUp, JANGAN salin kode, aset, ikon, nama, atau merek dagangnya.

## 2. Prinsip (urutan prioritas)
1. Benar dan presisi sebelum cepat. Geometri yang salah merusak semuanya.
2. Reuse library matang sebelum menulis sendiri (lihat §5).
3. Sederhana, kecil, mudah dipahami. Pemilik tidak mereview kode kernel baris per baris (K2, 2026-11-01). Karena itu kebenaran dijaga oleh test otomatis, review silang agen lain (§12), dan uji beta oleh 3D engineer. Ringkasan PR tetap wajib bisa dipahami pemilik.
4. Setiap perilaku punya test otomatis. Tanpa test, fitur dianggap belum ada.
5. Jangan mengarang. Jika tidak tahu, baca kode/dokumen, cari sumber, atau tanya manusia.

## 3. Stack
- Bahasa inti: Rust (ADR-001)
- UI: egui/eframe (ADR-002)
- Rendering: wgpu, batching per tile, RTC (ADR-003)
- Kernel geometri: crate `kernel` Hew, dependensi git dipin ke satu commit. Manifold opsional untuk boolean berat/LOD3 (ADR-004)
- Format native: `.hew` + atribut `wangungis.*` (ADR-006)
- Scripting/plugin: Python sebagai proses terpisah, mulai v0.5. WASM dipertimbangkan setelah v0.5 (ADR-007)
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
Jalur A2: `core-geometry` (half-edge, push/pull, boolean, toleransi), `commands` (undo/redo), dan `inference` disediakan crate Hew. Modul kita hanya membungkus atau menambah yang belum ada. Jangan menulis ulang yang sudah ada di Hew (§8 langkah 3a). Struktur final ditetapkan saat scaffold F0.

Setiap modul punya `SPEC.md` (tujuan, API publik, invariant, non-goals). Ubah kode publik = ubah SPEC.md di PR yang sama.

## 5. Reuse vs tulis sendiri
Wajib pakai library matang (jangan tulis ulang):
- Kernel/booleans/mesh ops: crate `kernel` Hew (ADR-004). Manifold opsional untuk boolean berat/LOD3; i_overlay/Clipper2 untuk poligon 2D.
- CRS: PROJ. Raster/vektor: GDAL. Point cloud: PDAL. IFC: IfcOpenShell.
- I/O mesh: assimp / glTF / OpenUSD. Validasi LOD: val3dity. CityJSON: cjio / citygml-tools sebagai referensi dan validator.

Memang harus ditulis sendiri (inti diferensiasi):
- Tool interaksi gaya SketchUp dan UI (egui), renderer wgpu, indeks inference per tile di atas crate `inference` Hew, origin proyek/CRS, LOD engine (roof fitting, template fasad), I/O geospasial.
- Half-edge, Push/Pull, Intersect, undo, dan inference dasar **tidak** ditulis ulang: pakai crate Hew (ADR-000 A2). Kekurangannya ditambal lewat PR upstream atau patch kecil di fork.

Menambah dependensi baru: tulis ADR singkat (lisensi kompatibel, ukuran, aktivitas proyek, alternatif). Dependensi proprietary/berbayar dilarang.

## 6. Aturan geometri
- Koordinat world: double precision. Gunakan floating origin / origin shifting untuk koordinat UTM besar.
- Predikat geometri kritis (orientasi, in-circle, bidang) memakai robust/exact predicates. Kernel Hew baru memakai orientasi 2D eksak; kode kita mengikuti aturan ini, dan kebutuhan 3D dievaluasi ulang (ADR-004). Dilarang membandingkan float dengan `==` atau epsilon ad hoc. Toleransi dipusatkan di satu modul dan didokumentasikan.
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
   3a. Cek apakah fitur sudah ada di crate Hew (jalur A2, ADR-000) atau di library terpilih sebelum menulis.
4. Tulis/ubah test dulu atau bersamaan. Jalankan test lokal.
5. `ponytail-review` (bila ada) untuk over-engineering.
6. Review silang agen lain khusus KEBENARAN (invariant, edge case, presisi), bukan hanya ukuran kode.
7. CI 3 OS hijau.
8. Update SPEC.md/ADR bila perlu. Tulis log sesi (§11).

## 9. Perintah (isi setelah scaffold)
- Build: `cargo build --workspace`
- Test semua: `cargo test --workspace` (CI juga menjalankan `--release`)
- Test satu modul: `cargo test -p wangungis-<modul>` (mis. `wangungis-scene`)
- Lint/format: `cargo fmt --all --check` dan `cargo clippy --workspace --all-targets -- -D warnings`
- Benchmark: `TODO`
- Validasi ekspor LOD (val3dity): `python scripts/validate_lod.py` (butuh `cjval` dan `val3dity` di PATH)

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
Wajib persetujuan manusia sebelum: menambah dependensi, mengganti commit pin crate Hew, mengubah format file native, mengubah API publik, mengubah toleransi geometri, menghapus test, atau perubahan lisensi. Review manusia rutin: tiap akhir fase + tiap 2 minggu baca ulang SPEC.md modul inti.

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
