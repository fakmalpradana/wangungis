# Tahap 4 — Alur Kerja Agen

Status: draf 2026-10-29. Aturan permanen tetap di `AGENTS.md`. Dokumen ini berisi peran, prompt siap pakai, konvensi, manajemen kuota, dan **usulan** perubahan AGENTS.md/CLAUDE.md/GEMINI.md yang menunggu persetujuan manusia.

## 1. Peran

| Peran | Claude Code | Gemini (CLI / agy) | Manusia |
|---|---|---|---|
| Perancang (SPEC, ADR) | ✓ utama | review | menyetujui |
| Implementasi `core-geometry`, `inference`, `tools`, `lod` | ✓ utama | review kebenaran | membaca ringkasan PR |
| Implementasi `io`, wrapper format, glue UI | review | ✓ utama | — |
| Penulis test tambahan / property test | review | ✓ utama | — |
| Benchmark dan laporan metrik | — | ✓ utama | membaca angka |
| Riset library (web) | sintesis + verifikasi | ✓ pengumpulan bahan | memutuskan |
| Review silang | untuk kode Gemini | untuk kode Claude | final |

Aturan: agen yang menulis kode tidak menjadi satu-satunya reviewer kodenya. Peran bisa ditukar per PR (AGENTS.md §12).

Catatan biaya (plugin antigravity): delegasi ke agy hanya untung untuk pekerjaan besar dan repetitif. Tugas kecil lebih murah dikerjakan langsung. Hasil agy selalu diverifikasi dengan menjalankan test sendiri di lingkungan bersih.

## 2. Alur per tugas (ringkas)

1. Baca `SPEC.md` modul + test terkait. Jangan membaca seluruh repo.
2. Rencana 5 baris (caveman).
3. Tangga ponytail: perlu ada? sudah ada di std/dependensi/Hew? fitur platform? baru tulis versi terkecil yang benar. Catat hasilnya di deskripsi PR.
4. Test dulu atau bersamaan. Jalankan lokal.
5. `/ponytail-review` untuk over-engineering; `/caveman-review` untuk komentar ringkas.
6. Review silang agen lain: invariant, edge case, presisi.
7. CI 3 OS hijau.
8. Update SPEC/ADR (bahasa normal). Tulis log sesi.

## 3. Prompt siap pakai

Ganti `<…>` sebelum dipakai. Semua prompt mengasumsikan protokol skill (AGENTS.md §7) sudah dijalankan di awal sesi.

### 3.1 Fitur baru (Claude Code)
```
Skill: caveman (status), ponytail lite untuk core-geometry/inference, full untuk lainnya.
Tugas: implementasi fitur #<nomor> "<nama>" dari docs/plan/feature-matrix.md di modul <modul>.
Baca dulu: <modul>/SPEC.md, test di <modul>/tests/, ADR terkait (<ADR>).
Langkah: (1) rencana 5 baris, (2) tangga ponytail dan catat hasilnya,
(3) tulis test unit + property test (input acak tidak boleh merusak invariant),
(4) implementasi minimal yang benar, (5) jalankan `<perintah test modul>`,
(6) update SPEC.md bila API publik berubah, (7) log sesi.
Batas: jangan ubah toleransi di core-geometry::tol, jangan tambah dependensi, jangan ubah format native tanpa bertanya.
Selesai bila: test lulus lokal, tidak ada test yang dihapus/dilemahkan, ringkasan PR 3–5 baris.
```

### 3.2 Bug
```
Skill: caveman (laporan debug), ponytail (patch tersempit).
Bug: <deskripsi + langkah reproduksi + file contoh>.
Langkah: (1) tulis test yang gagal dan mereproduksi bug, (2) cari akar masalah dengan bukti
(jangan menebak), (3) perbaiki di lapisan tersempit yang bertanggung jawab, (4) pastikan test lain tetap lulus,
(5) bila bug geometri: tambahkan kasus ke property test agar kelas bug ini tertangkap.
Laporan: akar masalah 1–2 kalimat, file:line, test yang ditambah.
```

### 3.3 Refactor
```
Skill: ponytail (hapus sebelum menambah), caveman.
Target: <modul/file>. Tujuan: <lebih sederhana / pisah tanggung jawab / hapus duplikasi>.
Syarat: perilaku tidak berubah. Jalankan seluruh test modul sebelum dan sesudah; hasil harus identik.
Hash replay deterministik tidak boleh berubah. Jangan ubah API publik kecuali disebut di sini.
Laporkan: baris kode sebelum/sesudah (tokei), dependensi sebelum/sesudah.
```

### 3.4 Test tambahan (Gemini)
```
Skill padanan: aturan ringkas di AGENTS.md; jangan mengurangi test yang ada.
Modul: <modul>. Baca SPEC.md bagian "Invariant".
Tulis: property test (proptest) untuk setiap invariant yang belum punya test,
dan kasus tepi: face degenerate, edge sangat pendek (< LENGTH_EPS), koordinat UTM besar, footprint self-intersect.
Jangan ubah kode produksi. Bila test menemukan bug, tandai #[ignore = "bug: <ringkas>"] dan laporkan; jangan perbaiki diam-diam.
```

### 3.5 Dokumentasi
```
Skill: caveman TIDAK dipakai untuk isi dokumen (bahasa normal penuh).
Tulis/perbarui: <SPEC.md / docs/user/... / README>. Sumber kebenaran: kode dan test saat ini.
Setiap klaim perilaku harus bisa ditunjuk ke test. Bahasa: Indonesia, istilah teknis Inggris.
```

### 3.6 Benchmark (Gemini)
```
Jalankan `<perintah benchmark>` pada main dan pada branch <branch>. Laporkan tabel median + p95 per benchmark,
selisih %, dan tandai regresi > <ambang>%. Jangan mengubah kode benchmark atau ambang.
Simpan hasil ke docs/metrics/<tanggal>-<branch>.md.
```

### 3.7 Riset library (agy atau Claude)
```
Pertanyaan: library apa untuk <kebutuhan>? Kandidat awal: <a, b, c>.
Untuk tiap kandidat: lisensi (cek file LICENSE), aktivitas (rilis & commit terakhir), ukuran dependensi,
dukungan macOS/Windows/Linux, contoh pemakaian nyata, risiko. Setiap klaim wajib URL.
Tanpa sumber = tandai "asumsi". Simpan ke docs/research/<topik>.md. Akhiri dengan rekomendasi + alternatif ditolak (siap jadi ADR).
```

### 3.8 Review silang kebenaran (agen lain)
```
Review diff <branch> khusus KEBENARAN, bukan gaya:
invariant half-edge (manifold, orientasi konsisten, tidak ada face degenerate), edge case,
presisi (perbandingan float, toleransi di luar core-geometry::tol, f32 di luar render),
error handling, dan apakah test benar-benar menguji klaimnya.
Format: path:line: severity: masalah. perbaikan. Tanpa pujian.
```

## 4. Konvensi git dan PR

- Branch: `feat/<modul>-<topik>`, `fix/…`, `refactor/…`, `docs/…`, `spike/…` (spike tidak di-merge ke main kecuali hasil tulisannya).
- Commit: Conventional Commits, kecil (`feat(inference): add midpoint snap`). `/caveman-commit` boleh dipakai.
- PR: ringkasan 3–5 baris, risiko, test yang ditambah, hasil benchmark bila relevan, catatan tangga ponytail.
- Main selalu hijau. Tidak ada push langsung ke main.

## 5. Menjaga basis kode tetap bisa dipahami

1. PR ≤ ±400 baris diff (di luar test dan fixture). Lebih besar = pecah.
2. Tes "jelaskan kembali": manusia menjelaskan perubahan dari ringkasan PR saja. Gagal = sederhanakan (AGENTS.md §10).
3. Setiap 2 minggu manusia membaca ulang `SPEC.md` modul inti (AGENTS.md §14).
4. Per fase: `/ponytail-audit` untuk over-engineering dan `/ponytail-debt` untuk menagih komentar `ponytail:`.

## 6. Manajemen konteks dan kuota

| Aturan | Alasan |
|---|---|
| Satu sesi = satu tugas kecil (satu fitur di matrix, satu bug) | Konteks kecil = biaya input kecil; input/cache mendominasi biaya, bukan output ([skill.md](../research/skill.md)) |
| Mulai dari `SPEC.md` + test modul, bukan seluruh repo | Idem |
| Log sesi ≤ 15 baris di `docs/sessions/` | Pengganti memori percakapan |
| Hasil riset disimpan di `docs/research/` | Tidak diulang |
| Bulk (test masal, migrasi, riset web luas) ke agy; review diff, bukan seluruh pohon | Hemat token Claude |
| Konteks hampir penuh → skill `context-handoff` | Sesi berikut mulai bersih |

Anggaran per fase (asumsi awal, dikoreksi dengan data nyata setelah Fase 0):

| Fase | Sesi Claude | Sesi Gemini/agy | Catatan |
|---|---|---|---|
| F0 | 10–15 | 5–10 | spike + CI |
| v0.1 | A: 5–10 · B: 40–60 | 10–20 | jalur A sebagian besar verifikasi |
| v0.2 | 20–30 | 15–25 | I/O dan CRS cocok untuk Gemini |
| v0.3 | 30–45 | 15–25 | atap + straight skeleton paling berat |
| v0.4 | 25–40 | 15–25 | fasad + multi-LOD |
| v0.5+ | 30–50 | 20–30 | paritas lainnya |

## 7. Usulan perubahan file aturan

Status 2026-10-29 (disetujui manusia): butir 7.1 nomor 3–6, 7.2, dan 7.3 **sudah diterapkan**. Butir 7.1 nomor 1–2 menunggu nama proyek dan hasil spike (ADR-000).

### 7.1 AGENTS.md
1. §1: isi nama proyek setelah dipilih; lisensi setelah ADR-005 diterima.
2. §3: isi stack dari ADR-001..004 setelah spike (ADR-000).
3. §7: ganti "catat … (`/ponytail-gain` bila ada)" dengan: "`/ponytail-gain` hanya menampilkan median benchmark plugin. Metrik proyek dicatat sendiri: `tokei`, `cargo tree`, waktu build CI, token per sesi."
4. §7: tambahkan: "caveman 4.0.1 dan ponytail 4.7.0 terpasang (diverifikasi 2026-10-28). Mode caveman: caveman/ultracave/megacave; ponytail: lite/full/ultra."
5. §8: tambahkan langkah 3a "cek apakah fitur sudah ada di Hew (jalur A) sebelum menulis."
6. §13: tambahkan tipe branch `spike/…`.

### 7.2 CLAUDE.md
- Tambahkan: "Skill review tersedia: `/ponytail-review`, `/caveman-review`, agen `caveman:cavecrew-reviewer`. Delegasi bulk ke agy lewat agen `antigravity:antigravity-delegate`; verifikasi sendiri hasilnya."

### 7.3 GEMINI.md (draf, belum dibuat di root)
```markdown
# GEMINI.md

@AGENTS.md

Semua aturan proyek ada di AGENTS.md. Tambahan khusus Gemini:
- Skill caveman/ponytail tidak tersedia sebagai plugin Gemini. Terapkan padanannya sebagai aturan:
  status ringkas (tanpa basa-basi, fakta utuh); kode minimal dengan tangga YAGNI → stdlib → dependensi ada → minimum.
  Pengecualian sama dengan AGENTS.md §7.
- Catat di log sesi bahwa padanan ini dipakai (tidak setara penuh).
- Jangan pernah mengubah lingkungan (paket terpasang, stub dependensi) agar test lulus.
```
Catatan: impor `@./file.md` di GEMINI.md didukung Gemini CLI 0.38.0 (dokumentasi `docs/reference/memport.md` dalam paket terpasang). File final memakai `@./AGENTS.md`.
