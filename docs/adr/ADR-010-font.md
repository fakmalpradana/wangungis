# ADR-010: Font Noto yang dibundel

- Status: **Accepted** 2026-11-25 (keputusan pemilik)

## Konteks
Font bawaan egui hanya mencakup huruf Latin dan Sirilik. Menu bahasa menawarkan 16 bahasa (Mandarin, Jepang, Korea, Arab, Thai, Hindi, dan lainnya), sehingga tanpa font tambahan teksnya tampil sebagai kotak kosong.

## Keputusan
- Font Noto dibundel di `ui/fonts/` dan ditanam ke biner dengan `include_bytes!`. Pemilik memilih ini dibanding font sistem dan dibanding subset.
- `ui/src/fonts.rs` memulai dari font bawaan egui, sehingga huruf Latin tampil seperti sebelumnya, lalu menambahkan Noto sebagai fallback pada keluarga Proportional dan Monospace, berurutan: Noto Sans, Arab, Devanagari, Thai, CJK.
- Dari koleksi CJK hanya satu wajah yang didaftarkan, sesuai bahasa: Ja 0 (JP), Ko 1 (KR), ZhHans 2 (SC), ZhHant 3 (TC), bahasa lain 2 (SC). Dengan begitu aksara Han memakai bentuk regional yang benar. Berganti bahasa memasang ulang font hanya bila wajahnya berubah. Byte font dipakai bersama (`from_static`), tidak disalin.
- Hanya bobot Regular, dalam TTF tanpa hinting. egui merender glyph sendiri dan tidak memakai hinting TrueType, jadi hinting hanya menambah ukuran. Bobot tebal tidak dipakai UI.

## Berkas
Semua berkas dibiarkan persis seperti rilisnya (tidak dimodifikasi, tidak diganti nama).

| Berkas | Versi | Sumber rilis | Ukuran (byte) | SHA-256 |
|---|---|---|---|---|
| NotoSans-Regular.ttf | NotoSans-v2.015 (latin-greek-cyrillic) | https://github.com/notofonts/latin-greek-cyrillic/releases/tag/NotoSans-v2.015 | 431 364 | `f3961a9cde016d41a4879aecda1474d3a36d6bf54fa0e4643de029cc2248b0e8` |
| NotoSansArabic-Regular.ttf | NotoSansArabic-v2.013 | https://github.com/notofonts/arabic/releases/tag/NotoSansArabic-v2.013 | 142 140 | `bd86ca02f087d7f3c3788ba458fb6b73744c7639ed276b8d870dba6def6c40d0` |
| NotoSansDevanagari-Regular.ttf | NotoSansDevanagari-v2.007 | https://github.com/notofonts/devanagari/releases/tag/NotoSansDevanagari-v2.007 | 184 228 | `9c7d935139ea6a1e6ad9dbac4f6d27ece1e04bca8123c8888d00a0f9df4724cd` |
| NotoSansThai-Regular.ttf | NotoSansThai-v2.002 | https://github.com/notofonts/thai/releases/tag/NotoSansThai-v2.002 | 20 960 | `d4303fe9c63ebb72759ca8b6d2040c8ae81689f7d08d7b91c656154382b49313` |
| NotoSansCJK-Regular.ttc | Sans2.004 (aset `03_NotoSansCJK-OTC.zip`) | https://github.com/notofonts/noto-cjk/releases/tag/Sans2.004 | 19 484 784 | `b76b0433203017ca80401b2ee0dd69350349871c4b19d504c34dbdd80541690a` |

Teks lisensi tiap keluarga ada di sebelah font: `OFL-NotoSans.txt`, `OFL-NotoSansArabic.txt`, `OFL-NotoSansDevanagari.txt`, `OFL-NotoSansThai.txt`, `OFL-NotoSansCJK.txt`. Indeks wajah CJK yang diverifikasi: 0 JP, 1 KR, 2 SC, 3 TC, 4 HK, 5 sampai 9 varian Mono.

## Lisensi
Semua font berlisensi SIL Open Font License 1.1. OFL mengizinkan font dibundel dan didistribusikan bersama perangkat lunak apa pun, termasuk yang berlisensi AGPL-3.0-only, selama font tidak dijual sendirian dan teks lisensinya ikut didistribusikan. Aturan Reserved Font Name hanya berlaku bagi font yang dimodifikasi; kita tidak memodifikasi apa pun. Font tetap berlisensi OFL, bukan AGPL. Distribusi biner dan paket wajib menyertakan folder `ui/fonts/` atau berkas OFL-nya.

## Alternatif ditolak
- **Font sistem**: tampilan berbeda di tiap OS, dan font yang dibutuhkan (mis. Thai, Devanagari, CJK) bisa tidak terpasang, terutama di Linux. Pemuatannya juga lebih rumit dan sulit diuji otomatis.
- **Subset** (hanya glyph yang dipakai katalog): biner jauh lebih kecil, tetapi butuh alat dan berkas turunan yang harus dibuat ulang setiap katalog atau bahasa bertambah, dan turunan font yang dimodifikasi terkena aturan Reserved Font Name. Pemilik memilih berkas asli yang tidak diubah.
- **Bobot lain atau font variabel**: UI tidak memakai teks tebal; menambah ukuran tanpa manfaat.

## Konsekuensi
- Ukuran repo dan biner bertambah sekitar 20 MB, hampir semuanya koleksi CJK (19,5 MB). Berkas biner ditandai `-text -diff` di `.gitattributes`.
- Memasang ulang font saat bahasa CJK berganti membuat ulang atlas glyph; byte font tidak disalin.
- Tes `ui/src/fonts.rs` memastikan setiap karakter di katalog dan nama bahasa punya glyph, urutan fallback, pemetaan wajah CJK, dan ukuran tiap berkas (berkas yang tertukar terdeteksi).
- Pembaruan versi font = ADR baru atau revisi ADR ini, dengan checkpoint manusia seperti dependensi lain.

## Hasil ukur
| Ukuran | Sebelum | Sesudah |
|---|---|---|
| Ukuran biner release (macOS arm64) | 19 178 096 B | 39 625 200 B |
| RSS 8 detik setelah start, bahasa `id` | 108-111 MB | 109-112 MB |
| RSS 8 detik setelah start, bahasa `ja` | - | 110-111 MB |

Diukur 2026-11-25 di Mac pengembang, tiga kali bergantian. RSS tidak berubah karena byte font berada di biner yang dipetakan dan glyph baru dirasterisasi saat dipakai. Waktu sampai frame pertama tidak diukur dengan angka; jendela terbuka tanpa jeda yang terasa berbeda. Terlihat di jendela asli: menu bahasa menampilkan ke-16 nama, termasuk aksara CJK, Hangul, Arab tersambung, Thai, dan Devanagari.

## Tambahan 2026-11-25: Arab di depan untuk bahasa Arab
egui memecah teks menjadi run per font. Bila font Latin ada di depan, spasi dalam kalimat Arab diambil dari font itu, sehingga setiap kata menjadi run sendiri dan kata-kata tersusun kiri-ke-kanan, terbalik dari seharusnya. Untuk bahasa Arab, Noto Sans Arabic dipasang paling depan, sehingga satu kalimat Arab menjadi satu run yang dibentuk kanan-ke-kiri. Terlihat benar di jendela asli. Kalimat yang mencampur huruf Latin atau angka masih bisa tertukar, karena egui belum punya bidi.
