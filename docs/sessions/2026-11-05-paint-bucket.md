# 2026-11-05 — Paint Bucket (#49)

Tujuan: PR 4 dari delapan fitur wajib v0.1.0.

Selesai: warna per verteks di `render` (ditulis Codex dari brief, diperiksa dan di-commit Claude), `Scene::paint`, `Tool::paint`, baris warna di UI.

Keputusan:
- Warna dasar model kini abu-abu bawaan Hew, sama dengan ekspor. Disetujui pemilik lewat rencana.
- Warna solid saja. Tanpa tekstur, transparansi, dan pengambil warna. Status matrix "parsial" karena itu.

Temuan: `add_material` Hew tidak bisa di-undo, jadi file sesudah undo cat bisa berbeda byte dari sebelum cat. Kriteria save, load, save byte-identik tidak terganggu. Test memeriksa isi cat, bukan byte.

Codex: sandbox-nya melarang tulis ke `.git`. Sandbox tidak dilonggarkan; Codex hanya mengedit berkas dan commit dilakukan konduktor.

Langkah berikut: PR 5, edit isi grup di tempat.
