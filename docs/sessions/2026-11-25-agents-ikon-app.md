# 2026-11-25 dokumen agen dan ikon aplikasi

- AGENTS.md: status rc.1, struktur repo nyata (7 crate), aturan teks UI lewat i18n, gerbang lokal dengan `WANGUNGIS_REQUIRE_GPU=1`, delegasi Sonnet, batas agy/Codex, CHANGELOG dan alur rilis, checkpoint tag rilis dan hapus branch.
- CLAUDE.md: delegasi Sonnet, review agy read-only oleh Claude sendiri, larangan polling CI, uji jendela asli di bundle scratchpad.
- GEMINI.md: agy headless hanya review, format jawaban.
- Ikon: `ui/assets/icon.svg` (sumber) + `icon-1024.png`, `icon-256.png`, `icon.ico`, `icon.icns`. Putih bersih, W dari garis hitam berujung bulat, goresan terakhir oranye `#F27300`, penanda snap titik ujung oranye di puncak tengah, garis dasar hitam-oranye. Terbaca di 32 px.
- Render: Quick Look (`qlmanage`) lalu sudut dibuat transparan dengan PIL; `.icns` via `iconutil`. Tidak ada dependensi baru.
- Belum: ikon belum dipasang ke jendela app maupun `.exe`/`.app` (perlu keputusan: dependensi resource Windows butuh ADR).
