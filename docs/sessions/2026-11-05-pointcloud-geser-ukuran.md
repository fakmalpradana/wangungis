# 2026-11-05 ukuran titik per awan, geser awan
Tujuan: permintaan pemilik: ukuran titik per awan, dan awan bisa digeser dan di-snap ke apa pun.
Selesai: `Cloud.point_size` dan `offset`, `DrawChunk` membawa keduanya, `Tool::cloud_move` (J), `Scene::move_cloud`, `Step` di riwayat undo, panel per awan, berkas pendamping dengan field baru.
Keputusan: tool terpisah dari M, bisa di-undo, LAS dan cache tidak berubah, origin proyek tidak berubah.
Keputusan: titik acuan lewat J memakai `cloud_under`. Lewat snap biasa, sumbu mengalahkan titik awan dan klik ditolak; ketahuan dari smoke.
Test: snap tujuan ke tiap jenis termasuk origin, tidak ke diri sendiri, Esc, undo berselang dengan edit, proptest, golden ukuran dan offset UTM, smoke sampai buka ulang.
Gagal: test piksel lama kena penanda snap di bawah kursor. Kursor kini keluar dulu; perbandingan tetap persis.
Review: agy bersih. Codex 3 temuan valid, diperbaiki: simpan di tengah geseran menulis posisi sementara, ukuran titik tidak memicu autosave, offset berkas pendamping melewati batas proyek.
Berikut: izin PR, CI, uji pemilik langkah 35-36.
