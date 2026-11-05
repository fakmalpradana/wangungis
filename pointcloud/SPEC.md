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

- `CloudSet`: awan-awan satu proyek, dimiliki `tools::Scene`. `add(file, source)`, `remove(id)`, `set_visible(id, on)`. Tiap frame: `poll()` mengambil hasil baca, `plan(&View)` memilih jumlah titik per chunk dan meminta yang kurang, `take_changed(f)` memberi chunk yang berubah untuk diunggah, `draws()` memberi daftar gambar. `busy()` berarti masih ada bacaan berjalan. `pick(origin, dir, aperture)` memberi id awan dan titik di bawah sinar. `bounds()` memberi batas lokal proyek.
- Per awan (`Cloud`): `point_size` (1 sampai 8 titik layar, lewat `set_point_size`) dan `offset` (meter dari posisinya di LAS, lewat `set_offset`, angka tidak finite ditolak). LAS dan cache tidak berubah; menggeser tidak mengunggah ulang.
- Pengaturan publik di `CloudSet`: `frame` (posisi dunia dari nol proyek), `draw_budget` (bawaan 2 juta titik), `load_budget` (bawaan 5 juta titik), `point_size` (ukuran untuk awan baru), `snap`, `slice`, `slice_on`, `moving` (awan yang sedang digeser, yang tidak bisa dipilih `pick`). `active_slice()` memberi kotak yang berlaku.
- Berkas pendamping menyimpan ukuran dan offset per awan. Berkas lama tanpa keduanya tetap terbaca: ukuran dari `point_size` berkas, offset nol.
- `saved(dir)`, `apply_saved(&SavedClouds)`, `saved_clouds_path(project)`: berkas pendamping `<proyek>.pointcloud.json`. Nilainya diperiksa dan dipotong ke rentang saat dibaca.

## Cara kerja LOD
Titik tiap chunk teracak, jadi awalan berapa pun panjangnya adalah sampel merata. `plan` menghitung luas chunk di layar, lalu jumlah titik yang menutupinya pada ukuran titik saat ini. Chunk di luar pandangan atau di luar kotak potong tidak digambar. Bila total melewati `draw_budget`, semua chunk dikurangi dengan porsi sama, sampai lantai per chunk. Pembacaan berjalan di satu thread, paling banyak 16 sekaligus, yang terdekat lebih dulu, dan dilebihkan sedikit agar gerakan kecil tidak menyentuh disk. Bila memori melewati `load_budget`, chunk dengan kelebihan terbesar dipotong. Satu chunk dimuat paling banyak `MAX_CHUNK_POINTS`, supaya buffer tidak melewati batas GPU.

## Hasil ukur `AQ_27.las` (Mac Apple Silicon, build release, file sudah di cache OS)
26.546.545 titik, 902 MB, 1 × 1 km, UTM 48S. Disk dingin belum diukur.

| Ukuran | Hasil | Target |
|---|---|---|
| Impor pertama lewat aplikasi | 1,43 s | ≤ 60 s |
| Buka lagi dari cache sampai tampil | 0,06 s | ≤ 2 s |
| Berkas cache | 319 MB, 289 chunk, chunk terbesar 218.094 titik | ±35% LAS |
| RAM proses tanpa jendela, penuh dan dekat (`rss`) | 104 MB dan 132 MB, dari 31 MB | tambahan ≤ 200 MB |
| RAM di jendela asli Retina, setelah impor | 184 MB, 0% CPU saat diam | |
| Titik di memori, dekat | 4,3 juta (±52 MB) | ≤ 5 juta |
| Frame saat orbit, tampilan penuh, 500 ribu titik, 1280 × 800 tanpa jendela | median 4,8 ms, terburuk 9,9 ms | ≤ 33 ms |
| Pilih titik di bawah kursor, dekat, 2 juta titik | rata-rata 0,83 ms, terburuk 1,39 ms | ≤ 5 ms |
| Potong dan tampilkan semua | 0,3 ms dan 0,4 ms per frame | satu frame |

Angka ini milik satu mesin. GPU terintegrasi lain belum diukur; anggaran bisa diturunkan dari panel.

## Invariant
- Setiap titik sumber berada tepat dalam satu chunk nonkosong.
- Batas cloud dan chunk adalah batas sebenarnya dari titik yang dibaca dan seluruh angkanya finite.
- Posisi disimpan sebagai `u16` relatif terhadap batas chunk; galat kuantisasi maksimum setengah langkah pada tiap sumbu.
- Rentang byte chunk berada setelah header dan sebelum tabel, tidak tumpang tindih, dan jumlah titik chunk sama dengan jumlah titik cloud.
- Buffer tulis bersama dibatasi 64 MiB. Pengacakan hanya memuat satu chunk pada satu waktu.
- Impor gagal, panik pembaca, dan pembatalan tidak meninggalkan berkas sementara.
- `plan` tidak pernah menggambar lebih dari yang dimuat. Memori tidak melewati `load_budget`, termasuk bacaan yang masih berjalan. `pick` hanya melihat titik yang sedang digambar dan ada di dalam kotak.

## Non-goals
- Membaca LAZ.
- Reproyeksi koordinat.
- Menyimpan klasifikasi, intensitas, waktu GPS, atau atribut LAS selain warna.
- Rendering: milik `render`.
