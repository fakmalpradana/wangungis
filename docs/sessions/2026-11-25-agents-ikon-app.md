# 2026-11-25 dokumen agen dan ikon aplikasi

- AGENTS.md: status rc.1, struktur repo nyata (7 crate), aturan teks UI lewat i18n, gerbang lokal dengan `WANGUNGIS_REQUIRE_GPU=1`, delegasi Sonnet, batas agy/Codex, CHANGELOG dan alur rilis, checkpoint tag rilis dan hapus branch.
- CLAUDE.md: delegasi Sonnet, review agy read-only oleh Claude sendiri, larangan polling CI, uji jendela asli di bundle scratchpad.
- GEMINI.md: agy headless hanya review, format jawaban.
- Ikon final (varian X, iterasi ke-5): W miring 12° dari dua goresan berbasis lingkaran (lengan lurus + setengah lingkaran, ujung bulat). U hitam di depan dengan pinggir putih; oranye satu lengan + sebagian lingkaran bawah, menyelip di belakang. Ujung luar melengkung (oranye ke kiri, hitam ke kanan). Oranye ±26% luas (diukur piksel), sesuai syarat pemilik 1:3. Kotak batas glyph di tengah tile (512,512). Ditolak: v1 garis lurus, v2 sketsa bebas, bilah, gelombang, bulan sabit.
- Render: Quick Look (`qlmanage`) lalu sudut dibuat transparan dengan PIL; `.icns` via `iconutil`. Tidak ada dependensi baru.
- Belum: ikon belum dipasang ke jendela app maupun `.exe`/`.app` (perlu keputusan: dependensi resource Windows butuh ADR).
