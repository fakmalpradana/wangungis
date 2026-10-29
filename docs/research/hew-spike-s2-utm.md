# Spike S2 — presisi push/pull Hew di koordinat UTM (kriteria #3)

Tanggal: 2026-10-29. Kernel: Hew v1.3.1 (`612c1c1`). Mesin: M4 16 GB. Sumber test: [`hew-spike-s2-utm.rs`](hew-spike-s2-utm.rs). File test disalin ke `crates/kernel/tests/` di clone Hew. Perintah:

```
cargo test [--release] -p kernel --test zz_wangungis_utm_spike -- --nocapture --test-threads=1
```

## Metode

Untuk setiap kasus:
- Buat kotak 5 × 4 × 3 m. Kotak diletakkan di titik acuan tertentu dan pada kasus tertentu diputar 30° di bidang XY.
- Jalankan 100 push/pull berurutan. Dua dari tiga operasi ada di face atas, satu dari tiga di face samping. Jaraknya 0,5–17 mm, bergantian tarik dan dorong.
- Galat dihitung terhadap posisi eksak. Jumlah jarak dicatat dalam bilangan bulat satuan 0,5 mm. Kotak harus tetap punya tepat 8 vertex, dan setiap vertex dicocokkan satu-satu ke sudut yang diharapkan. Galat = jarak Euclid terbesar di kerangka lokal kotak.
- Revisi 2026-10-29: metrik ini menggantikan metrik "bidang terdekat" versi pertama, setelah review silang Gemini. Kesimpulannya tidak berubah.
- Setelah itu: gambar persegi 1 × 1 m di face atas, angkat 0,5 m (boss), lalu undo semua langkah dan bandingkan dengan keadaan awal.

## Hasil (build release)

| Kasus | Push/pull berhasil | Ditolak | Galat vertex maks (m) | Gambar + boss | Galat undo (m) |
|---|---|---|---|---|---|
| Origin, sejajar sumbu | 100 | 0 | 1,3e-15 | ok | 0 |
| UTM (430000, 9150000), sejajar sumbu | 100 | 0 | 3,4e-11 | ok | 0 |
| UTM + pecahan, sejajar sumbu | 100 | 0 | 3,4e-11 | ok | 0 |
| Origin, diputar 30° | 100 | 0 | 3,1e-15 | ok | 0 |
| Easting saja (430000, 0), diputar 30° | 100 | 0 | 5,4e-11 | ok | 0 |
| **Northing saja (0, 9150000), diputar 30°** | **97** | **3** | 7,1e-10 | ok | 0 |
| **UTM + pecahan, diputar 30°** | **87** | **13** | 1,5e-9 | ok | 0 |
| Lokal (100, 200), diputar 30° | 100 | 0 | 2,1e-14 | ok | 0 |
| 10 km (10000, 10000), diputar 30° | 100 | 0 | 4,1e-13 | ok | 0 |

Semua penolakan berupa `PushPullError::NonManifoldResult`.

Catatan debug build: hasil debug **tidak sama** dengan release. Pada dua kasus yang ditebalkan, push/pull pertama yang gagal memicu panic `kernel invariant violated after mutation: face ... has vertices off its stored plane`. Panic terjadi di `check_invariants` pada **salinan** Object (`ops.rs:3038`), yaitu sebelum hasil disimpan. Di build release, kasus yang sama menjadi penolakan bertipe. Object asli tidak berubah, jadi tidak ada model yang rusak diam-diam.

## Kesimpulan

1. Operasi yang **diterima** selalu presisi. Galat terbesar 1,5e-9 m, jauh di bawah syarat ≤ 1 mm.
2. Geometri yang **tidak sejajar sumbu**, pada northing UTM Indonesia sebagai koordinat absolut, membuat 3–13% push/pull **ditolak**. Penyebab yang paling mungkin: toleransi absolut `PLANE_DIST = POINT_MERGE = 1e-9` m, sementara ulp `f64` di Y ≈ 9,15e6 sudah ≈ 1,86e-9 m. Bidang face yang dihitung ulang tidak bisa memuat vertex-nya sendiri dalam toleransi itu. Geometri sejajar sumbu lolos karena normalnya eksak (0, 0, 1) atau (1, 0, 0).
3. Dengan **koordinat lokal**, baik dekat origin maupun pada jarak 10 km, tidak ada penolakan sama sekali.

**Status kriteria #3: belum lulus.** Bagian kernel lulus bersyarat, tetapi bagian render belum diuji. Selain itu, UI Hew belum punya lapisan origin-shift: origin yang disimpan di attribute dictionary tidak dipakai oleh kernel maupun viewport. Jadi pada jalur A, lapisan itu harus dibuat. Rotasi hanya diuji pada 30°. Rentang sudut lain dan radius tile maksimum belum di-proptest.

Rincian bagian kernel, lulus bersyarat: Syaratnya: WangunGIS menyimpan geometri relatif terhadap origin proyek `f64`, dan koordinat UTM absolut tidak pernah masuk ke kernel. Ini sudah menjadi rancangan ADR-003 dan ADR-006. Bagian render (jitter di viewport three.js) belum diuji karena butuh GUI; ini dilanjutkan bersama spike FPS (Hari 7).

Usulan untuk upstream: toleransi relatif terhadap besaran koordinat, atau dokumentasi bahwa kernel menganggap geometri berada dekat origin. Usulan ini dimasukkan ke draf issue Hew (Hari 5).
