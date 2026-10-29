# Riset nama proyek (K3)

Tanggal cek: 2026-10-29. Alasan: "citySketch" mengandung kata "Sketch" (risiko merek R12). Cek dilakukan lewat API registri (`crates.io`, PyPI, npm), pencarian repo GitHub, dan pencarian web. **Belum** ada cek basis data merek resmi (PDKI/DJKI, USPTO, EUIPO). Cek itu wajib sebelum nama dipakai publik.

Kode status: 404 = nama paket kosong, 200 = nama paket sudah dipakai.

| Nama | Makna | crates.io | PyPI | npm | Repo GitHub bernama sama | Catatan |
|---|---|---|---|---|---|---|
| **Wangun** | Jawa: bentuk/rupa, konsep estetika ([ISI Surakarta](https://jurnal.isi-ska.ac.id/index.php/acintya/article/download/4341/3254/12699)) | 404 | 404 | 404 | 45, bintang tertinggi 5 | Tidak ada software 3D bernama Wangun di web. Akun GitHub `wangun` sudah ada. |
| **Undak** | Bertingkat (punden berundak); cocok dengan tingkatan LOD1→LOD3 | 404 | 404 | 404 | 10, bintang tertinggi 15 | Di web hanya muncul sebagai istilah *undak-usuk* basa Sunda. Akun `undak` sudah ada. |
| **Pahat** | Memahat; cocok dengan push/pull | 404 | 404 | 404 | 33, bintang tertinggi 1 | Ada repo kecil `PAHAT-Lang`. Akun `pahat` sudah ada. |
| **Rancang** | *Rancang bangun* = desain teknik | 404 | 404 | 404 | 711 (repo tugas kuliah) | Kata umum; sulit dicari di mesin pencari. |
| **Gedung** | Bangunan | 404 | 404 | 404 | 410 (repo tugas grafkom) | Terlalu literal. |
| **Bangun** | *Bangun ruang* = benda 3D; juga "membangun" | 404 | 404 | 404 | 3.946 | Kata sangat umum; nyaris tidak bisa dicari. |

Ditolak:
- Ruang: bentrok dengan merek Ruangguru, dan ada 8.595 repo bernama sama.
- Parapet, Kriya, Atap: nama paket di crates.io, PyPI, dan npm sudah dipakai.
- Gable: PyPI dan npm sudah dipakai.

Perintah cek (ulang bila perlu): `curl https://crates.io/api/v1/crates/<nama>`, `curl https://pypi.org/pypi/<nama>/json`, `curl https://registry.npmjs.org/<nama>`, `gh api "search/repositories?q=<nama>+in:name"`.

Akun GitHub bernama sama bukan penghalang. Repo bisa ditaruh di akun pemilik (`<akun>/wangun`), tetapi nama organisasi `wangun` tidak tersedia.

## Keputusan
2026-10-29: manusia memilih **WangunGIS**, yaitu Wangun + GIS sebagai penanda fokus geospasial. Hasil cek pada hari yang sama:
- `wangungis`, `wangun-gis`, dan `wangun_gis` masih kosong di crates.io, PyPI, dan npm.
- Belum ada repo GitHub bernama `wangungis` (0 repo).
- Nama organisasi GitHub `wangungis` masih tersedia (HTTP 404).

Cek merek resmi (PDKI, TMview) belum dilakukan.

## Rekomendasi awal
1. **Wangun**: pendek, unik di semua registri, maknanya "bentuk", dan berakar budaya Jawa (dekat dengan Yogyakarta).
2. **Undak**: metafora LOD bertingkat paling kuat, dan juga unik di registri.

Sebelum dipakai publik: cek PDKI (pdki-indonesia.dgip.go.id) kelas 9/42, dan TMview (tmdn.org/tmview) untuk merek internasional.
