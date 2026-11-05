# 2026-11-05 — Push/Pull dengan Ctrl (#7)

Tujuan: PR 3 dari delapan fitur wajib v0.1.0.

Selesai: `Scene::extrude_face_new`, `keep` di `State::PushPull`, `Tool::keeping`, label "Solid baru" di status bar.

Keputusan:
- Hasilnya solid terpisah yang berimpit dengan solid asal, mengikuti Hew. Disetujui pemilik lewat rencana. Status matrix "parsial" karena SketchUp menambah face pada satu mesh.
- Ctrl atau C juga berlaku pada Push/Pull, sama dengan Move dan Rotate.

Test: test tool, proptest jarak acak positif dan negatif, smoke tap Ctrl dan undo.

Langkah berikut: PR 4, sisi `tools` dan `ui` dari Paint Bucket.
