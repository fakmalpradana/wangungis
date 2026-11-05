# 2026-11-05 — Review Gemini PR #12

- Tujuan: review silang lintas model yang tertunda untuk PR #12. Kuota agy habis, jalan setelah reset.
- Gemini timeout 20 menit tanpa output; `--conversation <id>` + "tulis laporan sekarang" berhasil. Pelajaran: minta laporan akhir lebih awal, atau beri batas waktu lebih panjang.
- 5 temuan, semua dicek ke kode dan sumber egui. Semua valid minimal sebagian.
- Paling serius: Line + kunci sumbu + kursor diam + ketik panjang = garis mengikuti arah lama, geometri salah. Diperbaiki di `tools` (kunci melupakan kursor) dan `ui` (hover ulang).
- Lain: Tab dari viewport masuk kotak path, lalu Enter membuka ulang file dan edit hilang. Cmd+S mati saat kotak fokus. Esc keluar kotak ikut membatalkan gesture. Tahan tombol navigasi = repaint terus. Klik tidak membuang nilai. Label "Dipecah" salah hitung setelah undo.
- Ditolak: "tile (0,0) menggelembungkan hitungan" (disengaja, terdokumentasi di `occupied`); saran L/R/P tanpa penjaga `typed` (test mengunci perilaku ini).
- Kutipan Gemini untuk `text_edit_focused` tidak ada di egui 0.37, klaim perilakunya tetap benar. Selalu cek kutipan.
- Keputusan: Tab tidak lagi mencapai kotak path; kotak hanya lewat klik. Trade-off aksesibilitas, tercatat di SPEC.
- Test regresi: 5 di `ui/src/smoke.rs`, 1 di `tools/tests/tools.rs`. Semua gagal dulu sebelum perbaikan.
- Belum teruji: klik nyata di jendela asli, `docs/user/uji-asap.md` masih untuk 3D engineer.
