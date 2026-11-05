# 2026-11-05 Push/Pull banyak target
Tujuan: Push/Pull untuk dua objek terpilih. Laporan uji pemilik, butir 2.
Selesai: `Scene::push_pull_many`, `Scene::matching_face`, `Tool::carry_selection`, Shift+klik di tool, `App::switch_tool` membawa seleksi.
Keputusan: dua cara masuk, Select lalu P, atau Shift+klik di tool. Sama-sama satu langkah undo, satu tile.
Keputusan: grup atau komponen ikut terpilih berarti ditolak dengan pesan, bukan dilewati diam-diam.
Terukur: id region dalam satu sketsa dan id face dalam satu solid bertahan setelah target lain dikerjakan, jadi tidak perlu resolve ulang.
Test: dua solid, dua region satu sketsa, dua face satu solid, toggle, rollback, lintas tile, grup, proptest jarak acak, replay `PullTwo`, smoke.
Gagal: tidak ada.
Berikut: PR 11, menu Berkas dan dialog OS.
