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
| Pilih / Garis / Persegi / Push/Pull / Pindah / Putar / Skala / Hapus | Spasi / L / R / P / M / Q / S / E, atau tombol di toolbar |
| Salin saat Pindah atau Putar | C atau Ctrl. Status bar menulis "Menyalin" |
| Isi angka | Langsung ketik, lalu Enter. Muncul di status bar sebagai "Nilai" |
| Batal | Esc |
| Undo / Redo | Cmd+Z / Shift+Cmd+Z (Ctrl di Windows dan Linux) |
| Simpan | Cmd+S atau tombol Simpan |
| Kunci sumbu saat Garis atau Pindah; sumbu putar | Panah kanan = X, kiri = Y, atas = Z |

Format angka: `5`, `2,5`, `5;4`, `5x4`, `250cm`, `1500mm`. Angka negatif untuk arah berlawanan. Tepat setelah menyalin: `x5` untuk lima salinan, `/5` untuk lima salinan yang mengisi jarak salinan pertama.

Warna penanda snap: hijau titik ujung, sian titik tengah, merah tepi, biru face.

## Langkah
1. Jalankan tanpa file. Terlihat tiga garis sumbu: merah X, hijau Y, biru Z.
2. Tekan **R**. Klik satu titik di tanah. Gerakkan kursor: persegi oranye mengikuti.
3. Ketik `5;4` lalu Enter. Persegi 5×4 m tergambar, dengan isian abu-abu.
4. Tekan **P**. Arahkan kursor ke dalam persegi: garis tepinya menyala oranye. Klik.
5. Ketik `3` lalu Enter. Kotak setinggi 3 m terbentuk. Status bar: 1 objek, 12 segitiga.
6. Tetap di **P**, klik face atas, gerakkan kursor naik-turun, klik lagi. Tinggi berubah mengikuti kursor.
7. **Cmd+Z** berkali-kali sampai kosong, lalu **Shift+Cmd+Z** sampai kotak kembali.
8. Tekan **L**. Klik empat titik membentuk segi empat, lalu klik titik pertama lagi. Rantai berakhir sendiri. Dekatkan kursor ke sudut: penanda hijau muncul dan titik menempel.
9. Isi path di kotak atas, misalnya `/tmp/uji.hew` (file yang belum ada). Tekan **Simpan**. Tekan **Baru**: layar kosong dan kotak path kosong. Isi path yang sama, tekan **Buka**: kotak yang sama muncul lagi. Simpan menolak menimpa file lain yang sudah ada.
10. Gambar persegi di dinding atau atap kotak: face-nya terbelah. Tekan **P**, klik face baru, tarik keluar atau dorong ke dalam. Dorong sampai tembus untuk membuat pintu.
11. **Tugas referensi v0.1.** Dari layar kosong: buat kotak 5×4×3 m, gambar persegi 1×2 m di dinding menyentuh lantai, dorong tembus jadi pintu. Tekan **M**, tekan **C**, klik kotak, tekan panah kanan, gerakkan kursor ke kanan, ketik `7` lalu Enter, ketik `x10` lalu Enter. Ada sebelas kotak berpintu. Satu **Cmd+Z** menghapus semua salinan. Simpan, buka ulang: sama.
12. Tekan **E**, arahkan ke garis pintu yang belum didorong, klik: garis hilang dan face menyatu. Klik tepi kotak: seluruh kotak hilang. Cmd+Z mengembalikannya.
13. Tekan **Q**, klik sudut kotak, klik arah nol, ketik `90` lalu Enter. Lalu **S**, klik kotak, ketik `2` lalu Enter: dua kali lebih besar.
14. Buka model kota besar kalau ada. Status bar menyebut "Dipecah menjadi N tile". Gambar sesuatu di dekat batas 500 m dari pusat.

## Yang diketahui belum ada
- Pratinjau Push/Pull berupa garis tepi, bukan solid.
- Belum ada dialog file.
- Baru dan Buka tidak memperingatkan perubahan yang belum disimpan.
