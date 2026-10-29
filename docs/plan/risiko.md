# Tahap 6 — Risiko

Status: draf 2026-10-29. Ditinjau di akhir setiap fase. Skala: Kemungkinan dan Dampak = Rendah/Sedang/Tinggi.

| # | Risiko | Kategori | Kemungkinan | Dampak | Mitigasi | Pemicu (tanda risiko terjadi) |
|---|---|---|---|---|---|---|
| R1 | **Push/Pull dan Intersect Faces tidak robust**: face degenerate, topologi rusak, kasus koplanar | Teknis | Tinggi | Tinggi | Predikat `robust`; toleransi terpusat; validator setelah setiap mutasi (debug) dan sebelum commit (release); property test + fuzz malam hari; boolean lewat Manifold; operasi gagal dengan error, tidak memperbaiki diam-diam | Fuzz menemukan crash/invariant rusak > 1 per minggu; laporan pengguna "model rusak" |
| R2 | **Drift presisi pada koordinat UTM besar** | Teknis | Sedang | Tinggi | `f64` di model, RTC di render, test ganda origin vs UTM untuk setiap operasi (arsitektur §6) | Test ganda berbeda > toleransi; jitter terlihat di golden-image UTM |
| R3 | **Performa model besar** (10M face) | Teknis | Sedang | Sedang | BVH, culling, instancing, LOD rendering; benchmark otomatis dengan ambang regresi; uji model besar mingguan | FPS 1M face < 60 atau 10M < 30; regresi benchmark > ambang |
| R4 | **Ketergantungan pada Hew** (jalur A): pengelola tunggal berhenti, menolak scope geospasial, atau mengubah arsitektur | Teknis/komunitas | Sedang | Tinggi | Kriteria spike #5–#6; kode geospasial di crate terpisah dengan antarmuka sempit; fork tetap sah di bawah AGPL; sinkronisasi upstream terjadwal per fase | Tidak ada commit upstream > 2 bulan; PR ditolak; konflik merge berulang |
| R5 | **Pemetaan face poligon ↔ mesh segitiga Manifold** merusak planaritas atau semantik | Teknis | Sedang | Sedang | Spike S3 sebelum fitur boolean; label semantik dibawa lewat ID face; fallback: boolean khusus untuk kasus opening sederhana | S3 gagal; face non-planar setelah boolean |
| R6 | **Kode agen yang tidak dipahami manusia** | Kualitas kode agen | Tinggi | Tinggi | PR ≤ ±400 baris; SPEC.md per modul; tes "jelaskan kembali"; review silang agen lain; baca ulang SPEC tiap 2 minggu | Manusia tidak bisa menjelaskan PR dari ringkasannya; bug berulang di modul yang sama |
| R7 | **Kode "terlalu minimal"**: ponytail memangkas validasi, error handling, atau toleransi | Skill | Sedang | Tinggi | ponytail `lite` di `core-geometry`/`inference`; pengecualian AGENTS.md §7; review silang fokus kebenaran; test tidak boleh berkurang (cek jumlah test di CI) | Jumlah test turun; validator dilewati; komentar `ponytail:` di jalur geometri kritis |
| R8 | **Packaging tanpa code signing berbayar** | Hukum/distribusi | Tinggi | Sedang | macOS: panduan "Open Anyway", tap Homebrew sendiri (tap resmi menolak unsigned mulai Sep 2026), `cargo install`; Windows: SignPath Foundation (syarat: semua komponen tanpa dual-license komersial, lihat ADR-005, jadi hindari Slint/CGAL/Qt atau konfirmasi dulu); CityGML via citygml-tools butuh JRE 17+; Linux: Flatpak/AppImage | Pengguna gagal instal; isu GitHub tentang "aplikasi rusak/tidak bisa dibuka" |
| R9 | **Kuota/biaya agen habis** di tengah fase | Kapasitas | Sedang | Sedang | Satu tugas kecil per sesi; konteks dari SPEC.md; log sesi; bulk ke agy; anggaran sesi per fase (prompts.md §6) | Kuota mingguan habis sebelum 70% target sesi |
| R10 | **Kapasitas manusia** (studi Glasgow, pekerjaan freelance) | Kapasitas | Tinggi | Sedang | Roadmap berbasis milestone; fase bisa dijeda tanpa kehilangan konteks (log sesi); prioritas Must saja saat renggang | Tidak ada sesi > 3 minggu |
| R11 | **Scope creep** (paritas SketchUp penuh sebelum USP LOD jadi) | Produk | Tinggi | Tinggi | MoSCoW per rilis; USP LOD di v0.2–v0.4 didahulukan; fitur baru harus masuk matrix dulu; non-goals tertulis | Fitur non-matrix muncul di PR; v0.3 tertunda karena fitur v0.5 |
| R12 | **Risiko IP/clean-room** (UX SketchUp, pembacaan `.skp` di Hew) | Hukum | Rendah | Tinggi | Tidak menyalin kode, aset, ikon, nama, atau dokumentasi SketchUp; tidak menjanjikan `.skp` sebagai fitur; tidak meniru Ruby API (ADR-007) | Klaim/surat dari pemilik merek; PR berisi aset dari SketchUp |
| R13 | **Kompatibilitas lisensi** dependensi (AGPL/GPL/LGPL/BSL) | Hukum | Rendah | Sedang | ADR-005; `cargo deny` untuk lisensi di CI; val3dity hanya sebagai proses | `cargo deny` gagal; dependensi baru tanpa ADR |
| R14 | **Agen memanipulasi lingkungan agar test lulus** (stub dependensi, patch paket) | Kualitas kode agen | Sedang | Tinggi | Gate dijalankan sendiri di lingkungan bersih (CI); lockfile; larangan eksplisit di GEMINI.md | Test lulus lokal agen tetapi gagal di CI; perubahan di luar file yang diminta |
| R15 | **Klaim skill tidak sesuai kenyataan** | Skill | Tinggi | Rendah | Ekspektasi realistis (skill.md); ukur sendiri (tokei, token per sesi) | Token per sesi tidak turun setelah skill aktif |
| R16 | **Data uji dan lisensinya** (footprint/DSM Indonesia) | Hukum/data | Sedang | Sedang | Mulai dengan 3D BAG (terbuka); data Indonesia hanya bila lisensinya jelas; data sintetis untuk benchmark | Dataset tanpa lisensi tertulis |

## Risiko teratas untuk dipantau di Fase 0

R1, R2, R4, R5 diuji langsung oleh spike S1–S3. R6 dan R7 dimulai sejak PR pertama.
