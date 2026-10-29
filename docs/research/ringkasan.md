# Ringkasan Riset Tahap 0 (0.F)

Status: Sesi 3 (2026-10-28). Detail dan sumber: [teknologi.md](teknologi.md), [kompetitor.md](kompetitor.md), [skill.md](skill.md). ADR: `docs/adr/`.

## Ringkasan eksekutif

Proyek ini ingin menggabungkan UX SketchUp dengan pemodelan bangunan LOD1–LOD3 yang presisi dan open source. Riset menemukan dua hal penting.

Pertama, **UX gaya SketchUp sudah tersedia di Hew**, modeler open source berbasis kernel half-edge Rust (AGPL-3.0-only, umur 3,5 bulan, satu pengelola). Menulis ulang fondasi yang sama dari nol membutuhkan berbulan-bulan kerja.

Kedua, **blok geospasial dan LOD kosong di semua tool**. Ini mencakup CRS, presisi UTM, semantik CityGML, CityJSON, val3dity, multi-LOD, dan roof fitting. SketchUp bergantung pada plugin berbayar atau plugin yang tidak terawat. Blender bergantung pada add-on yang diarsipkan. CityEngine kuat secara prosedural, tetapi tertutup, berbayar, dan tidak mengekspor CityGML. Celah ini adalah USP proyek.

Keputusan manusia: **spike evaluasi Hew 1–2 minggu** (ADR-000) dengan 7 kriteria terukur. Hasilnya menentukan jalur A (bangun di atas Hew, AGPL) atau B (dari nol, GPL-3.0-or-later).

Stack yang direkomendasikan untuk kedua jalur:
- **Rust** sebagai bahasa inti; Python untuk plugin (proses terpisah).
- Half-edge + `robust` + Manifold + i_overlay/Clipper2.
- Model `f64`, render `f32` relatif ke pusat lokal (RTC).
- PROJ, cjval/cjseq, tyler, val3dity (CLI di CI).

Untuk distribusi tanpa biaya:
- macOS: panduan "Open Anyway" + tap Homebrew sendiri. Tap resmi menolak cask yang tidak ditandatangani mulai September 2026.
- Windows: SignPath Foundation.
- Linux: Flatpak/AppImage.

Skill caveman dan ponytail berguna sebagai aturan gaya. Penghematannya realistis sekitar 10–20% token output, bukan 65–75% seperti klaim README.

## Rekomendasi stack dan ADR

| ADR | Topik | Rekomendasi | Alternatif ditolak |
|---|---|---|---|
| 000 | Strategi Hew | Spike evaluasi → A atau B (**Accepted**) | — |
| 001 | Bahasa | Rust (+ Python plugin) | C++, Python penuh |
| 002 | UI | A: Tauri+React · B: egui | Slint, Qt/CXX-Qt, Dear ImGui |
| 003 | Render | A: three.js · B: wgpu; RTC untuk presisi | bgfx, API native langsung |
| 004 | Kernel | Half-edge + robust + Manifold + i_overlay | CGAL (default), OCCT, Fornjot |
| 005 | Lisensi | A: AGPL-3.0-only · B: GPL-3.0-or-later | MIT/Apache, MPL |
| 006 | Format native | Zip + manifest JSON berversi + buffer biner; CityJSON round-trip | CityJSON native, SQLite |
| 007 | Plugin | Python proses terpisah; tidak meniru Ruby API | WASM (ditunda), embedding default |

## Risiko utama (rinci di Tahap 6)

1. **Ketergantungan pada Hew** (jalur A): bus factor 1, proyek muda, pengelola bisa menolak scope geospasial. Mitigasi: kriteria spike #5–#6; fork tetap mungkin karena AGPL.
2. **Push/Pull dan Intersect pada koordinat besar**: drift presisi. Mitigasi: RTC, property test pada origin vs UTM.
3. **Pemetaan face poligon ↔ mesh Manifold**: planaritas hilang. Mitigasi: spike khusus sebelum fitur boolean.
4. **Distribusi tanpa signing di macOS**: friksi instalasi untuk pemula. Mitigasi: dokumentasi bergambar; tinjau ulang bila proyek mendapat sponsor.
5. **Kode agen yang tidak dipahami**: mitigasi SPEC.md per modul, review silang, ukuran PR kecil.

## Pertanyaan terbuka

1. Hasil spike Hew (ADR-000).
2. Lisensi `tyler`, citygml-tools, egui, Tauri.
3. Status AccessKit di macOS/Linux.
4. Packaging runtime Python tanpa signing.
5. Sumber data uji: 3D BAG (NL) tersedia dalam CityJSON; data Indonesia (mis. footprint DKI) perlu dicek lisensinya.

## Definisi metrik "setara" dan "unggul"

Setiap fitur di feature matrix (Tahap 2) diberi status berdasarkan uji yang bisa diulang:

| Status | Definisi |
|---|---|
| belum | Tidak ada, atau ada tanpa test otomatis |
| parsial | Ada dengan test; tetapi tugas referensi gagal atau hanya sebagian alur yang didukung |
| setara | Tugas referensi selesai dengan hasil sama dan waktu ≤ 1,2× SketchUp (diukur oleh pengguna yang sama, 3 kali, median) |
| unggul | Setara, **dan** salah satu: waktu ≤ 0,7× SketchUp; atau hasil lolos validasi (val3dity/watertight) yang di SketchUp gagal; atau fitur tidak ada di SketchUp |

Metrik global vs SketchUp (protokol rinci di Tahap 1):
- waktu pemula membuat rumah LOD2 dari footprint;
- FPS orbit pada 1M dan 10M face di M4 16 GB;
- waktu buka/simpan dan RAM pada model 1M face;
- galat posisi setelah 100 operasi pada koordinat UTM (target ≤ 1 mm);
- bangunan per jam (manual vs template vs otomatis);
- persentase ekspor yang lolos val3dity (target 100% untuk LOD1/LOD2).
