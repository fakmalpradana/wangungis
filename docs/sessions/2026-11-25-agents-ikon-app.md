# 2026-11-25 dokumen agen dan ikon aplikasi

- AGENTS.md: status rc.1, struktur repo nyata (7 crate), aturan teks UI lewat i18n, gerbang lokal dengan `WANGUNGIS_REQUIRE_GPU=1`, delegasi Sonnet, batas agy/Codex, CHANGELOG dan alur rilis, checkpoint tag rilis dan hapus branch.
- CLAUDE.md: delegasi Sonnet, review agy read-only oleh Claude sendiri, larangan polling CI, uji jendela asli di bundle scratchpad.
- GEMINI.md: agy headless hanya review, format jawaban.
- Ikon: `ui/assets/icon.svg` (sumber) + `icon-1024.png`, `icon-256.png`, `icon.ico`, `icon.icns`. Versi 2 dari sketsa pemilik: W abstrak melengkung di atas putih bersih. Kiri bilah oranye `#F27300` meruncing ke bawah, kanan satu bentuk hitam dengan taji tengah tajam dan lengan kanan membulat, gradien halus seperti logo referensi. Versi 1 (garis lurus + penanda snap) ditolak. Terbaca di 32 px.
- Render: Quick Look (`qlmanage`) lalu sudut dibuat transparan dengan PIL; `.icns` via `iconutil`. Tidak ada dependensi baru.
- Belum: ikon belum dipasang ke jendela app maupun `.exe`/`.app` (perlu keputusan: dependensi resource Windows butuh ADR).
