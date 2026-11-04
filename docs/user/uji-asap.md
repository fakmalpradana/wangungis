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
| Pilih / Garis / Persegi / Push/Pull | Spasi / L / R / P, atau tombol di toolbar |
| Isi angka | Langsung ketik, lalu Enter. Muncul di status bar sebagai "Nilai" |
| Batal | Esc |
| Undo / Redo | Cmd+Z / Shift+Cmd+Z (Ctrl di Windows dan Linux) |
| Simpan | Cmd+S atau tombol Simpan |
| Kunci sumbu saat Garis | Panah kanan = X, kiri = Y, atas = Z |

Format angka: `5`, `2,5`, `5;4`, `5x4`, `250cm`, `1500mm`. Angka negatif untuk arah berlawanan.

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
11. Buka model kota besar kalau ada. Status bar menyebut "Dipecah menjadi N tile". Gambar sesuatu di dekat batas 500 m dari pusat.

## Yang diketahui belum ada
- Pratinjau Push/Pull berupa garis tepi, bukan solid.
- Belum ada dialog file.
- Baru dan Buka tidak memperingatkan perubahan yang belum disimpan.
