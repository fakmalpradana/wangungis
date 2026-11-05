# 2026-11-05 snap Push/Pull
Tujuan: Push/Pull bisa snap ke geometri lain. Laporan uji pemilik, butir 1.
Akar: setelah face dipilih, tool hanya memakai `along_line`; `scene.snap` tidak dipanggil, penanda juga tidak muncul.
Selesai: `pull_to` di `tools/src/tool.rs`, dipakai hover dan klik. Mode Ctrl ikut.
Keputusan: snap memberi tinggi saja. OnFace hanya bila bidangnya sejajar. Jarak nol ditolak sebagai snap, supaya bidang sendiri tidak menahan.
Keputusan: OnGuide diabaikan sesuai rencana; bisa ditambah bila pemilik butuh tinggi dari guide.
Test: sudut, tengah tepi, tepi, face sejajar, turun, Ctrl, bidang sendiri, dinding, ketik menang, proptest tinggi acak, smoke App.
Gagal: tidak ada.
Berikut: PR 10, Push/Pull banyak target.
