# 2026-11-05 Move, auto-split, tool dasar, autosave

Tujuan: kriteria keluar v0.1 + uji UI nyata. Pemilik beri akses penuh hingga 05.00, tanpa bertanya.

Selesai:
- Uji UI headless `ui/src/smoke.rs`: App asli, event egui mentah, GPU offscreen. PR #13.
- Gambar lewat `Document::draft` Hew: auto-split face, isian region. PR #14.
- Move, Rotate, Scale, Eraser, Circle, Polygon, Arc, Offset; autosave; tanya sebelum buang. PR #15.
- Crate `io`: OBJ, STL, GLB, 3MF. Tampilan standar.
- `tools/tests/replay.rs`: 10.002 sesi acak lulus.

Bug ketahuan: klik tanah segaris sumbu Z mulai gambar di udara (uji headless); stack undo proyek menyimpang (proptest + Gemini); plus 8 temuan Gemini.

Keputusan: `Scene` undo simpan (tile, langkah Hew). Eraser: tepi pembagi face hilang sendiri, tepi asli hapus objek. Bidang gambar ikut kursor sampai segmen pertama. C = salin di Move/Rotate, Lingkaran di tool lain.

Gagal: akses computer-use ke app ditolak 2×, klik manusia belum teruji. Jangan akali lewat tool lain. Gemini: pakai `--model "Gemini 4 Argon (High)" --timeout 20m`, tanpa `--dir` repo (minta izin perintah lalu gagal).

Berikut: Group/Component (#42–#44), Tape (#28), Follow Me (#8), Tags/Outliner, rilis v0.1.0 (keputusan pemilik).
