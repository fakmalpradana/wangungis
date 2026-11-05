# SPEC — `pointcloud` (wangungis-pointcloud)

## Tujuan
Mengimpor LAS menjadi cache ringan yang dapat dibaca per area dan per tingkat detail. Titik dibagi dengan grid dua dimensi lalu diacak secara deterministik, sehingga prefix sebuah chunk menjadi sampel yang tersebar.

## API publik
- `cache_path(source, cache_dir)` membuat nama cache dari path kanonik, ukuran, dan waktu modifikasi sumber.
- `import_las(source, cache, progress)` memindai LAS, menulis cache sementara, mengacak setiap chunk, lalu mengganti cache tujuan secara atomik.
- `Progress` melaporkan fase, jumlah selesai, jumlah total, dan menerima pembatalan antarth-batch atau antarchunk.
- `CloudFile::open(cache)` memvalidasi seluruh metadata sebelum memercayai offset atau jumlah titik.
- `CloudFile::read(chunk, start, count)` membaca bagian chunk tanpa melewati ujungnya.
- `ChunkInfo::world(point)` mengubah posisi kuantisasi menjadi koordinat dunia.

## Invariant
- Setiap titik sumber berada tepat dalam satu chunk nonkosong.
- Batas cloud dan chunk adalah batas sebenarnya dari titik yang dibaca dan seluruh angkanya finite.
- Posisi disimpan sebagai `u16` relatif terhadap batas chunk; galat kuantisasi maksimum setengah langkah pada tiap sumbu.
- Rentang byte chunk berada setelah header dan sebelum tabel, tidak tumpang tindih, dan jumlah titik chunk sama dengan jumlah titik cloud.
- Buffer tulis bersama dibatasi 64 MiB. Pengacakan hanya memuat satu chunk pada satu waktu.
- Impor gagal, panik pembaca, dan pembatalan tidak meninggalkan berkas sementara.

## Non-goals
- Membaca LAZ.
- Reproyeksi koordinat.
- Menyimpan klasifikasi, intensitas, waktu GPS, atau atribut LAS selain warna.
- Rendering, pemilihan titik, dan LOD GPU.
