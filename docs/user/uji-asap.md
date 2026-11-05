# Uji asap WangunGIS

Untuk 3D engineer. Tujuannya memeriksa alur dasar: gambar, Push/Pull, undo, simpan. Catat apa yang terasa salah atau membingungkan, dengan langkah untuk mengulanginya.

## Menjalankan
Butuh [Rust](https://rustup.rs). Dari folder repo:

```bash
cargo run --release -p wangungis
```

Untuk membuka file: tambahkan `-- path/ke/file.hew`.

## Kontrol
| Aksi | Cara |
|---|---|
| Orbit | Seret tombol tengah, atau Alt + seret kiri (trackpad) |
| Pan | Shift + seret tombol tengah, atau Shift + Alt + seret kiri |
| Zoom | Scroll |
| Pilih / Garis / Persegi / Lingkaran / Poligon / Busur | Spasi / L / R / C / G / A |
| Push/Pull / Offset / Pindah / Putar / Skala / Cat / Hapus | P / F / M / Q / S / B / E, atau tombol di toolbar |
| Salin saat Pindah atau Putar | C atau tap Ctrl. Status bar menulis "Menyalin" |
| Solid baru saat Push/Pull | C atau tap Ctrl. Status bar menulis "Solid baru" (di tool lain, C berarti Lingkaran) |
| Masuk ke grup atau komponen | Klik dua kali dengan Pilih, atau tombol **Edit isi** |
| Keluar dari grup atau komponen | Esc saat tidak ada yang dibatalkan, atau tombol **Keluar** |
| Isi angka | Langsung ketik, lalu Enter. Muncul di status bar sebagai "Nilai" |
| Batal | Esc |
| Undo / Redo | Cmd+Z / Shift+Cmd+Z (Ctrl di Windows dan Linux) |
| Simpan | Cmd+S atau menu Berkas |
| Buka | Cmd+O atau menu Berkas |
| Simpan sebagai | Shift+Cmd+S atau menu Berkas |
| Kunci sumbu saat Garis atau Pindah; sumbu putar | Panah kanan = X, kiri = Y, atas = Z. Saat Garis, sumbu boleh keluar dari bidang gambar |
| Garis tegak lurus bidang gambar (keluar dari dinding) | Panah bawah |
| Garis miring | Ketik `30d` lalu Enter: 30° dari sumbu terkunci (tanpa kunci: dari X, atau horizontal di dinding), ke arah kursor. Lalu klik atau ketik panjang |

Baris di atas status bar selalu menulis apa yang ditunggu tool.

Kunci berlaku untuk satu segmen, lalu lepas sendiri. Sumbu yang keluar dari bidang gambar memakai bidang sumbu yang paling menghadap kamera.

Format angka: `5`, `2,5`, `5;4`, `5x4`, `250cm`, `1500mm`, `10ft`, `30d`. Angka tanpa satuan mengikuti satuan yang dipilih di status bar. Angka negatif untuk arah berlawanan. Tepat setelah menyalin: `x5` untuk lima salinan, `/5` untuk lima salinan yang mengisi jarak salinan pertama.

Warna penanda snap: hijau titik ujung, sian titik tengah, merah tepi, biru face, merah muda panduan dan sejajar atau tegak lurus. Nama jenis snap muncul di dekat kursor.

## Langkah
1. Jalankan tanpa file. Terlihat tiga garis sumbu: merah X, hijau Y, biru Z.
2. Tekan **R**. Klik satu titik di tanah. Gerakkan kursor: persegi oranye mengikuti.
3. Ketik `5;4` lalu Enter. Persegi 5×4 m tergambar, dengan isian abu-abu.
4. Tekan **P**. Arahkan kursor ke dalam persegi: garis tepinya menyala oranye. Klik.
5. Ketik `3` lalu Enter. Kotak setinggi 3 m terbentuk. Status bar: 1 objek, 12 segitiga.
6. Tetap di **P**, klik face atas, gerakkan kursor naik-turun, klik lagi. Tinggi berubah mengikuti kursor.
7. **Cmd+Z** berkali-kali sampai kosong, lalu **Shift+Cmd+Z** sampai kotak kembali.
8. Tekan **L**. Klik empat titik membentuk segi empat, lalu klik titik pertama lagi. Rantai berakhir sendiri. Dekatkan kursor ke sudut: penanda hijau muncul dan titik menempel.
9. Tekan **Cmd+S**: jendela penyimpanan OS terbuka. Pilih folder dan nama, misalnya `uji`. Buka menu **Berkas > Baru**: layar kosong. **Berkas > Buka**, cari file tadi: kotak yang sama muncul lagi. Buka **Berkas > Format yang didukung** dan baca apakah penjelasannya cukup jelas.
10. Gambar persegi di dinding atau atap kotak: face-nya terbelah. Tekan **P**, klik face baru, tarik keluar atau dorong ke dalam. Dorong sampai tembus untuk membuat pintu.
11. **Tugas referensi v0.1.** Dari layar kosong: buat kotak 5×4×3 m, gambar persegi 1×2 m di dinding menyentuh lantai, dorong tembus jadi pintu. Tekan **M**, tekan **C**, klik kotak, tekan panah kanan, gerakkan kursor ke kanan, ketik `7` lalu Enter, ketik `x10` lalu Enter. Ada sebelas kotak berpintu. Satu **Cmd+Z** menghapus semua salinan. Simpan, buka ulang: sama.
12. Tekan **E**, arahkan ke garis pintu yang belum didorong, klik: garis hilang dan face menyatu. Klik tepi kotak: seluruh kotak hilang. Cmd+Z mengembalikannya.
13. Tekan **Q**, klik sudut kotak, klik arah nol, ketik `90` lalu Enter. Lalu **S**, klik kotak, ketik `2` lalu Enter: dua kali lebih besar.
14. Tekan **C**, klik pusat, ketik `2` lalu Enter: lingkaran terisi, bisa di-Push/Pull jadi silinder. **G**, ketik `8s`, klik pusat, klik radius: segi delapan. **A**, klik dua ujung, gerakkan kursor, klik: busur. **F**, klik region, ketik `0,5` lalu Enter: tepi sejajar.
15. **Berkas > Ekspor**, pilih jenis `.obj` (atau `.stl`, `.glb`, `.3mf`) di jendela OS, simpan, buka di Blender. Skala dan posisinya harus sama.
16. Coba tombol tampilan: Atas, Depan, Kanan, Iso.
17. Tekan **Spasi**, klik kotak, Shift + klik kotak lain, tekan **Grup**. Tekan **M**, geser: keduanya ikut. **Pecah** mengembalikan. **Komponen** lalu salin dengan **M** dan **C**: salinannya berbagi geometri. **Delete** menghapus yang terpilih.
18. **Garis panduan.** Tekan **T**. Klik tepi bawah kotak (penanda merah atau sian). Gerakkan kursor menjauh: garis oranye sepanjang tepi mengikuti, dan status bar menulis jarak tegak lurus dari tepi. Ketik `2` lalu Enter: muncul garis merah muda sepanjang ±200 m, dan status bar menulis "Panduan dibuat". Tekan **R**, dekatkan kursor ke garis itu: titik menempel. **Hapus panduan** menghapusnya.
19. **Follow Me.**
    1. Tekan **Zoom extents** supaya kotak besar di layar. Tekan **R**, klik di dinding kotak, jauh dari tepinya, ketik `1;1` lalu Enter: persegi kecil.
    2. Tekan **L**, klik di tengah persegi. Tekan **panah bawah**. Baris petunjuk menulis "Terkunci tegak lurus bidang".
    3. Gerakkan kursor menjauh dari dinding, ketik `3` lalu Enter. Tekan **Esc**.
    4. Klik **Follow Me**, klik persegi (tepinya menyala oranye), klik garis: terbentuk batang menyatu dengan kotak. **Cmd+Z** membatalkannya.
    5. Di tempat kosong, tekan **L**, klik tanah, tekan **panah atas**, ketik `2` lalu Enter: garis tegak. **Esc**. Klik tanah lagi, gerakkan kursor, ketik `30d` lalu Enter, ketik `2` lalu Enter: garis miring 30° dari X.
20. **Dua tile, tanpa file besar.** Tekan **Baru**. Tekan **R**, klik dekat titik potong sumbu, gerakkan kursor ke arah sumbu X merah, ketik `800;10` lalu Enter. Tekan **Zoom extents**. Status bar: 1 tile. Klik tanah kosong di dekat ujung jauh, ketik `10;10` lalu Enter. Status bar: 2 tile. Tekan **L**, klik persegi kecil itu, dekatkan kursor ke sudut persegi panjang: penanda hijau muncul walau beda tile.
21. **Opsional, hanya bila punya model kota.** Buka filenya. Status bar menyebut "Dipecah menjadi N tile". Gambar sesuatu, lalu Cmd+Z. Lewati langkah ini bila tidak punya file.
22. **Satuan.** Di status bar, ganti `m` menjadi `mm`. Tekan **R**, klik tanah, ketik `5000;4000` lalu Enter: persegi 5×4 m. Tekan **T** dan ukur tepinya: status bar menulis `5000 mm`. Kembalikan ke `m`. Tutup dan buka lagi aplikasinya: pilihan satuan tetap diingat.
23. **Tooltip dan sejajar.** Tekan **L** dan gerakkan kursor ke sudut kotak: dekat kursor tertulis "Titik ujung". Gambar satu garis miring di tanah, lalu Esc. Mulai garis baru di tempat lain. Lewatkan kursor di atas garis miring tadi, lalu arahkan kursor searah dengannya: garis pratinjau menjadi merah muda dan tertulis "Sejajar tepi". Klik. Segmen berikutnya bisa dibuat "Tegak lurus tepi" terhadap segmen itu. Panjang yang diketik mengikuti arah itu. Periksa apakah inferensi ini membantu atau justru mengganggu saat Anda menggambar bebas.
24. **Solid baru dari face.** Tekan **P**, tap **Ctrl** (status bar: "Solid baru"), klik atap kotak, ketik `2` lalu Enter. Terbentuk solid kedua di atas kotak, dan kotak asal tidak berubah; dengan **M** keduanya bisa dipindah sendiri-sendiri. Ini berbeda dari SketchUp, yang menambah face pada satu benda. Cmd+Z membatalkannya sekali jalan.
25. **Cat.** Tekan **B**. Baris warna muncul di bawah toolbar. Pilih Bata, klik atap: atap berwarna, tetapi sisi lain tidak. Pilih warna lain lewat pemilih warna, klik dinding. **Hapus cat**, klik atap: warnanya hilang. Cmd+Z, simpan, buka lagi: warna tetap ada. Ekspor `.glb`, buka di Blender: warnanya ikut. Model tanpa cat kini abu-abu, bukan krem: nilai apakah tampilan ini nyaman.
26. **Edit isi grup.**
    1. Buat dua kotak berjauhan. Tekan **Spasi**, klik kotak pertama, tekan **Grup**.
    2. Tekan **P** dan klik atap kotak yang digrup: tidak ada yang terjadi, karena isinya tertutup.
    3. Tekan **Spasi**, klik dua kali kotak itu. Status bar menulis "Mengedit: Grup", dan kotak kedua memudar.
    4. Tekan **P**, klik atap, ketik `1` lalu Enter: atap naik. Coba Push/Pull atau hapus kotak kedua: tidak bisa.
    5. Tekan **Esc**: grup tertutup lagi dan kotak kedua kembali normal. Tekan **M**, geser grup: isinya ikut dengan bentuk baru. Cmd+Z berjalan mundur melewati tutup, edit, dan buka.
27. **Edit isi komponen.** Buat komponen dari satu kotak, salin dengan **M**, lalu klik dua kali salah satunya. Salinan lainnya hilang dari layar selama mengedit; ini batas v0.1. Tarik atapnya, tekan **Esc**: kedua salinan ikut berubah.
28. **Tags.** Di panel kanan, bagian Tags: pilih kotak, isi "Lantai 1" lalu **Buat dan beri**. Hilangkan centangnya: kotak hilang dan tidak bisa di-snap. Centang lagi. Coba **Lepas**, **Ubah nama**, **Hapus**, dan simpan lalu buka ulang.
29. **Outliner.** Di panel kanan, bagian Outliner: setiap objek, grup, dan komponen punya satu baris. Klik **+** pada grup untuk melihat isinya. Klik baris untuk memilih, klik dua kali untuk masuk. Hilangkan centang untuk menyembunyikan. Pilih baris, isi nama, tekan **Ubah nama**. Bila punya model kota, buka dan periksa apakah panel tetap ringan.

## Pengaman data
Baru, Buka, dan menutup jendela menanyakan dulu bila ada perubahan belum disimpan. Autosave berjalan tiap 60 detik ke `~/.wangungis/autosave.bin`. Setelah crash, aplikasi menawarkan pemulihan saat dibuka lagi.

## Yang diketahui belum ada
- Pratinjau Push/Pull berupa garis tepi, bukan solid.
- Belum ada dialog file.
- Selama komponen dibuka, salinan lainnya hilang dari layar sampai ditutup.
- Tidak bisa menggambar pada bidang yang berisi gambar di sisi lain batas grup. Misalnya, garis bebas di dalam grup yang menempel di tanah menghalangi garis baru di tanah di luar grup. Aplikasi menolak dengan pesan.
- Cat hanya warna solid, tanpa tekstur dan transparansi. Tag belum punya warna. Tinggal satu presisi per satuan, dan ft hanya desimal.
- Pada proyek banyak tile, saat grup dibuka, tile lain tidak memudar.
- Status bar menulis "Grup" untuk grup yang sedang dibuka, walau grup itu punya nama.
