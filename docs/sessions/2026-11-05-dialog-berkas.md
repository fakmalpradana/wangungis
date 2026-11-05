# 2026-11-05 menu Berkas dan jendela OS
Tujuan: buka, simpan, ekspor lewat jendela pencari berkas, dengan format dijelaskan. Laporan uji pemilik, butir 3.
Selesai: `rfd` (ADR-008), menu Berkas, jendela Format, `App::open_path`, `save_to`, `export_to`, `answer_dialog`. Kotak path dihapus.
Keputusan: jendela dipanggil di akhir frame. Timpa file lewat konfirmasi OS. Nama yang diubah aplikasi tidak pernah menimpa.
Test dihapus atas persetujuan pemilik: bagian fokus kotak path di `the_keyboard_and_the_path_box`. Bagian Tab tetap ada.
Test ditambah: batal menyimpan, lanjut setelah simpan, ekspor lima format, format tak dikenal, daftar format cocok dengan `wangungis_io::FORMATS`.
Belum terbukti: jendela asli (uji manual setelah PR 13), pesan Linux tanpa portal.
Berikut: PR 12, crate `pointcloud`.
