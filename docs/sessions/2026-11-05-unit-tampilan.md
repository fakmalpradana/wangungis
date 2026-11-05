# 2026-11-05 — Unit tampilan (#27)

Tujuan: PR 1 dari delapan fitur wajib v0.1.0.

Selesai: `vcb::Unit`, `parse_in`, `Unit::format`; `Tool::set_unit`; combo di status bar; `~/.wangungis/unit`.

Keputusan:
- Preferensi per pengguna, bukan per proyek. Format file tidak berubah. Disetujui pemilik lewat rencana.
- `parse` tetap ada untuk nilai yang bukan panjang (skala, derajat), `parse_in` untuk panjang. Diff lebih kecil daripada mengubah tanda tangan `parse` seperti di rencana.
- Presisi tetap per satuan, tanpa pengaturan. Status matrix "parsial" karena itu dan karena ft hanya desimal.

Test: tabel parser per satuan, proptest parser, test tool bahwa skala dan derajat tidak terskala, smoke mm lalu `10ft`.

Langkah berikut: PR 2, tooltip snap dan inference paralel.
