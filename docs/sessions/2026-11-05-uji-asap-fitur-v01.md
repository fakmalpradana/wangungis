# 2026-11-05 — Penutup delapan fitur v0.1.0

Tujuan: PR 8. Replay untuk operasi baru, langkah uji pemilik, roadmap, matrix.

Selesai: `tools/tests/replay.rs` kini mencakup solid baru, cat, grup, komponen, masuk dan keluar sesi, tag, sembunyikan tag, dan nama. Uji asap langkah 22-29. Status roadmap. Koreksi matrix #1 dan #36.

Keputusan:
- Undo sampai habis tetap harus memberi proyek kosong. Palet dan daftar tag Hew tidak ikut di-undo, jadi perbandingan byte hanya berlaku bila sesi tidak menyentuh keduanya; selain itu isinya yang dibandingkan.
- Sesi ditutup sebelum menyimpan di replay, karena file tidak pernah menyimpan sesi terbuka.

Temuan: pengukuran cakupan menunjukkan operasi `Enter` hampir tidak pernah membuka grup, jadi 10.000 sesi pertama nyaris tidak menguji sesi edit. Ditambah `EnterAny` dan `GroupAny`. Sesudahnya, pada 1.000 kasus, puluhan edit nyata terjadi di dalam sesi terbuka. Lulus 10.000 kasus lagi.

Pelaksana: agy tidak bisa menulis tanpa `--yolo`, Codex kena batas pemakaian, jadi Claude yang menulis. agy tetap mereview.

Langkah berikut: pemilik menguji langkah 22-29, lalu PR persiapan rilis.
