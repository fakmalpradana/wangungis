# RENCANA WangunGIS — Ringkasan dan Indeks

Status: 2026-10-29. Nama produk: **WangunGIS** (K3, 2026-10-29). "citySketch" adalah nama kerja lama yang masih dipakai di beberapa dokumen dan folder.

## 1. Ringkasan eksekutif

**Tujuan.** Aplikasi desktop open source dengan UX gaya SketchUp. Fokus awalnya pemodelan bangunan LOD1–LOD3 yang cepat, presisi, tergeoreferensi, dan valid. Aplikasi berjalan di macOS, Windows, dan Linux dari satu basis kode. Pelaksananya satu orang dibantu agen AI, tanpa budget uang.

**Temuan utama riset.**
1. UX gaya SketchUp sudah ada sebagai open source: **Hew** (kernel half-edge Rust, AGPL-3.0-only, umur 3,5 bulan, satu pengelola).
2. Blok geospasial dan LOD kosong di semua tool yang diteliti (11 tool, 66 fitur). Blok ini mencakup CRS dan presisi UTM, semantik CityGML, CityJSON, val3dity, multi-LOD, dan roof fitting.
3. Pipeline otomatis (3dfier, roofer/3D BAG) sudah menyelesaikan mayoritas bangunan secara batch. Celah terbesarnya adalah **editor interaktif untuk mengoreksi hasil yang gagal** dan **LOD3**.

**Posisi produk.** Editor LOD gaya SketchUp yang menjadi pelengkap pipeline otomatis. Model valid sejak digambar dan bekerja langsung di koordinat UTM.

**Keputusan yang sudah diambil.** Spike evaluasi Hew 1–2 minggu dengan 7 kriteria terukur (ADR-000). Hasilnya menentukan:
- **Jalur A**: bangun di atas Hew, lisensi AGPL. Estimasi likely ±540 jam manusia sampai v0.5.
- **Jalur B**: mulai dari nol, lisensi GPL-3.0-or-later. Estimasi likely ±740 jam.

**Stack yang direkomendasikan (ADR Proposed).**
- Bahasa: Rust; Python untuk plugin (proses terpisah).
- Geometri: half-edge + predikat `robust` + Manifold + i_overlay.
- Presisi: model `f64`, render RTC.
- Format dan validasi: PROJ, cjval/cjseq, tyler, val3dity di CI.
- UI dan render: Tauri+React+three.js (jalur A) atau egui+wgpu (jalur B).

**Distribusi gratis.**
- macOS: panduan "Open Anyway" + tap Homebrew sendiri.
- Windows: SignPath Foundation.
- Linux: Flatpak/AppImage.

**Ekspektasi skill.** caveman dan ponytail realistisnya menghemat ±10–20% token output, bukan 65–75%. Metrik proyek diukur sendiri.

**Risiko terbesar.**
- Robustness Push/Pull dan Intersect (R1).
- Drift koordinat besar (R2).
- Ketergantungan pada Hew (R4).
- Kode agen yang tidak dipahami (R6).
- Scope creep (R11).

Keempat risiko pertama diuji di Fase 0.

## 2. Indeks dokumen

| Tahap | Dokumen |
|---|---|
| 0 — Riset | [research/ringkasan.md](research/ringkasan.md) · [teknologi.md](research/teknologi.md) · [kompetitor.md](research/kompetitor.md) · [skill.md](research/skill.md) · bahan mentah [_raw-painpoints.md](research/_raw-painpoints.md) |
| ADR | [adr/](adr/) — ADR-000 (Accepted) s.d. ADR-007 (Proposed) |
| 1 — Produk | [plan/produk.md](plan/produk.md) |
| 2 — Feature matrix | [plan/feature-matrix.md](plan/feature-matrix.md) (100 fitur) |
| 3 — Arsitektur | [plan/arsitektur.md](plan/arsitektur.md) |
| 4 — Alur agen + prompt + usulan AGENTS/CLAUDE/GEMINI | [agents/prompts.md](agents/prompts.md) |
| 5 — Roadmap | [plan/roadmap.md](plan/roadmap.md) |
| 6 — Risiko | [plan/risiko.md](plan/risiko.md) |
| Log sesi | [sessions/](sessions/) |

## 3. Langkah 14 hari pertama

**Progres per 2026-10-29 (sebagian besar dikerjakan agen dalam mode otonom):**
- Hari 1–2 selesai: K1 AGPL, K3 WangunGIS, repo, CI.
- Hari 3–11: spike selesai kecuali yang butuh manusia.
  - #4: uji pemahaman.
  - #6: posting issue.
- Tambahan di luar rencana: opsi **A2** dengan 4 prototipe pendukung.
- Bahan keputusan Hari 12 (K2): [`research/keputusan-k2.md`](research/keputusan-k2.md).

Asumsi ritme ±1–2 jam per hari. Setiap hari = satu sesi agen kecil + log sesi.

| Hari | Kegiatan | Hasil yang bisa dicek |
|---|---|---|
| 1 | Baca `RENCANA.md`, `ringkasan.md`, ADR-000. Putuskan nama kerja dan apakah AGPL bisa diterima (kriteria spike #7). Terapkan usulan perubahan AGENTS.md/CLAUDE.md (prompts.md §7) bila setuju. | AGENTS.md diperbarui; keputusan AGPL tertulis |
| 2 | `git init`, branch `docs/rencana-awal`, commit `docs/`. Buat repo GitHub publik dan pasang `brew install cmake val3dity` (`brew tap tudelft3d/software` dulu). | Repo publik dengan dokumen rencana |
| 3 | Clone Hew di folder terpisah. Jalankan build + test-nya. Catat waktu dan masalah (kriteria #1). | Log sesi S1-a |
| 4 | Baca `ARCHITECTURE.md` + kernel push/pull Hew. Tulis penjelasan 1 halaman dengan bahasa sendiri (kriteria #4). | `docs/research/hew-kernel.md` |
| 5 | Buka issue/diskusi di Hew: perkenalkan scope geospasial/LOD, tanyakan apakah PR atau crate ekstensi akan diterima (kriteria #6). | URL issue |
| 6 | Spike S2 di Hew: buat model di X=430.000, Y=9.150.000; ukur galat push/pull dan amati render (kriteria #3). | Angka galat + tangkapan layar |
| 7 | Spike FPS: model sintetis 1M face di Hew desktop (kriteria #2). Hasil uji Hari 3–7 dicatat di ADR-000. | Angka FPS |
| 8 | CI GitHub Actions 3 OS untuk repo kita (workflow `cargo test` minimal + val3dity). | CI hijau |
| 9 | Spike S4: kubus LOD1 → CityJSON → `cjval` + `val3dity` di CI. | Job CI lulus |
| 10 | Spike S3: boolean Manifold pada face poligon (bangunan L + opening jendela); property test 1.000 kasus. | Test lulus/gagal + catatan |
| 11 | Evaluasi titik ekstensi Hew: dapatkah crate `geo`/`lod` ditambahkan tanpa mengubah kernel (kriteria #5)? Prototipe kecil: atribut CRS di dokumen. | Diff prototipe |
| 12 | Isi tabel kriteria ADR-000 dengan angka. Putuskan **jalur A atau B**. | ADR-000 diperbarui |
| 13 | Perbarui ADR-001..007 menjadi Accepted/direvisi sesuai jalur; isi `TODO` di AGENTS.md §1, §3, §9. | ADR Accepted; AGENTS.md tanpa TODO stack |
| 14 | Retrospektif Fase 0 bagian awal: jam terpakai vs estimasi, koreksi roadmap; pilih 3 fitur pertama v0.1/v0.2. Minta review silang Gemini atas ADR dan roadmap. | `docs/sessions/…-retro-f0.md`; roadmap dikoreksi |

## 4. Keputusan yang perlu Anda ambil

| # | Keputusan | Kapan | Rekomendasi |
|---|---|---|---|
| K1 | Apakah AGPL-3.0-only bisa diterima? (syarat jalur A) | Hari 1 | **Diputuskan 2026-10-29: Ya.** Kriteria spike #7 lulus. |
| K2 | Jalur A atau B setelah spike | Hari 12 | Ikuti kriteria ADR-000 apa adanya |
| K3 | Nama proyek (bebas merek "SketchUp"/"Sketch") | Hari 1–14 | **Diputuskan 2026-10-29: WangunGIS** (`docs/research/nama.md`). Cek merek PDKI/TMview masih harus dilakukan. |
| K4 | Setujui usulan perubahan AGENTS.md, CLAUDE.md, dan pembuatan GEMINI.md | Hari 1 | Setujui (prompts.md §7) |
| K5 | Ritme kerja per semester (5/10/20 jam per minggu) untuk mengunci estimasi | Hari 14 | Mulai dengan 5–10 jam dan ukur |
| K6 | Toleransi geometri awal (`LENGTH_EPS`, `PLANARITY_EPS`) | Setelah spike S2 | Ditentukan dari data spike, bukan ditebak |
| K7 | Dataset uji Indonesia yang lisensinya jelas | Sebelum v0.2 | Mulai dengan 3D BAG; cari data terbuka DKI/BIG |
| K8 | Perbaiki config agy `tier_pro` → "Gemini 4 Argon (High)" | Kapan saja | Ya; delegasi `--tier pro` sekarang gagal |

## 5. Glosarium

| Istilah | Arti |
|---|---|
| LOD0–LOD3 | Level of Detail model kota (CityGML): LOD1 balok extrude, LOD2 dengan bentuk atap, LOD3 dengan detail fasad (jendela, pintu, balkon) |
| CityGML / CityJSON | Standar OGC untuk model kota 3D; CityJSON adalah encoding JSON yang ringkas |
| 3D Tiles | Standar OGC untuk streaming data 3D besar (Cesium) |
| val3dity | Validator geometri 3D sesuai ISO 19107 (TU Delft) |
| cjval / cjseq | Validator skema dan alat stream CityJSON (Rust) |
| Half-edge | Struktur data mesh yang menyimpan setiap edge sebagai dua setengah-edge berlawanan arah; memudahkan traversal topologi |
| Manifold / watertight | Mesh tertutup tanpa lubang, setiap edge dimiliki tepat dua face |
| Robust predicates | Uji geometri (mis. orientasi) yang hasilnya pasti benar meski memakai floating point (Shewchuk) |
| RTC | Relative-to-center: render relatif terhadap pusat lokal agar float32 GPU tetap presisi |
| Floating origin | Memindahkan origin kerja mendekati kamera/objek untuk menjaga presisi |
| CRS / EPSG | Sistem referensi koordinat dan kodenya (mis. EPSG:32749 = UTM 49S) |
| Inference | Mesin snapping gaya SketchUp: endpoint, midpoint, sumbu, paralel |
| VCB | Value Control Box: kotak input angka saat menggambar |
| Push/Pull | Tool yang mengekstrusi face tegak lurus |
| Straight skeleton | Algoritma untuk membentuk atap hip/gable dari footprint poligon |
| Roof fitting | Mencocokkan bidang atap ke data DSM/point cloud |
| Spike | Eksperimen kecil berbatas waktu untuk membuktikan risiko teknis |
| ADR | Architecture Decision Record |
| SPEC.md | Spesifikasi modul: tujuan, API publik, invariant, non-goals |
| MoSCoW | Prioritas Must/Should/Could/Won't |
| caveman / ponytail | Skill Claude Code: output ringkas / kode minimal |
| agy | Antigravity CLI (Gemini) untuk delegasi pekerjaan bulk |
