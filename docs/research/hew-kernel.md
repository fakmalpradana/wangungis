# Kernel Hew: alur push/pull (spike S1, kriteria #4)

Tanggal baca: 2026-10-29. Sumber yang dibaca: Hew v1.3.1 (`612c1c1`), `docs/dev/ARCHITECTURE.md`, dan kode di `crates/wasm-api`, `crates/kernel`. Nomor baris di bawah merujuk ke commit tersebut.

## Gambaran besar

Kernel Hew adalah satu crate Rust murni, `kernel`, sekitar 65 ribu baris. Kernel ini dikompilasi ke WebAssembly dan **berjalan sebagai WASM di semua platform, termasuk desktop**. UI React memanggil kernel lewat satu-satunya pintu, yaitu `wasm-api`. Kernel tidak boleh bergantung pada UI, I/O, atau jaringan. Dependensinya hanya `slotmap`, `serde`, `serde_json`, `zip`, dan `tracing`.

Satu dokumen berisi pohon **Object**, Group, Component, Sketch, dan Guide. Setiap Object adalah solid **watertight** dengan mesh half-edge sendiri. Geometri hanya "lengket" (memotong atau bergabung) di dalam satu Object. Dua Object yang bersentuhan tetap dua Object. Penggabungan selalu eksplisit lewat boolean atau Group.

## Alur push/pull, dari klik sampai mesh berubah

1. **`Scene::push_pull`** di `crates/wasm-api/src/lib.rs:6686`. Pintu masuk dari UI dengan argumen Object, face, dan jarak. Ada tiga kemungkinan arah:
   - Bila dorongan menembus dinding seberang (`push_pull_overshoots`), operasi diubah menjadi boolean lewat `Document::push_pull_through` (`document.rs:12593`). Dorong ke dalam menjadi *subtract*, dan bisa membelah solid menjadi dua Object. Tarik keluar menjadi *union*.
   - Bila face adalah sub-face datar yang digambar di dalam Object, operasi menjadi `KernelOp::ExtrudeSubFace` (boss atau recess).
   - Selain itu, operasinya `KernelOp::PushPull`.
2. **`Document::apply_object_op`** di `document.rs:11654`. Fungsi ini memastikan Object terlihat dan berada di world. Ia menyiapkan garis sketch yang "ikut" face yang didorong, lalu meneruskan operasi ke riwayat undo milik Object. Setelah itu ia mencatat satu langkah undo di tingkat dokumen dan menjalankan validasi debug.
3. **`History::apply`** di `history.rs:368`. Sebelum operasi jalan, fungsi ini merekam bukti keadaan awal (`StateProof`, berupa sidik geometri). Ia lalu menjalankan operasi, menurunkan operasi kebalikannya dari laporan hasil, dan menyimpan "jangkar" geometris. Jangkar ini membuat undo tetap menemukan face yang benar walaupun ID-nya berubah.
4. **`Object::push_pull`** di `ops.rs:2651`, inti operasinya, berjalan dalam 7 langkah:
   1. Validasi masukan: Object harus watertight, face harus ada, dan |jarak| ≥ `POINT_MERGE`. Face lengkung (silinder) diarahkan ke `offset_cylinder_wall`, yang mengubah jari-jari seluruh dinding.
   2. Setiap tepi face diklasifikasi menurut sudut antara normal face dan normal tetangganya:
      - Tegak lurus (*transverse*): tetangga cukup memanjang.
      - Sebidang atau miring: tepi dilepas (*unweld*), lalu dibangun dinding quad baru.
   3. Untuk dorongan ke dalam, diperiksa apakah solid akan habis (`WouldVanish`).
   4. **Clone, ubah, validasi, tukar** (`ops.rs:2947`). Semua perubahan dikerjakan pada salinan Object.
      - Jalur cepat: semua tepi tegak lurus, misalnya atas kotak. Cukup geser vertex dan hitung ulang bidang face.
      - Jalur dinding (`push_pull_build_walls`) diikuti `validate_sweep_result` untuk menolak self-intersection.
      - Jalur *collapse* menutup kembali undakan sebidang.
   5. Validasi akhir pada salinan:
      - `check_invariants` hanya aktif di build debug, dan langsung panic bila invariant rusak.
      - `validate` selalu jalan, juga di build release, dan menolak dengan error bertipe `NonManifoldResult`.
      - Object asli tidak tersentuh dalam kedua kasus.
   6. Salinan menggantikan Object asli.
   7. Laporan hasil memuat face yang dibuat dan yang dihapus, serta penanda apakah undo butuh `unbuild_push_pull`. Undo khusus ini diperlukan untuk dinding miring, karena dorongan `-d` biasa tidak bisa mengembalikannya.

**Undo** (`history.rs`, `dispatch_replay`) menjalankan operasi kebalikan pada salinan, dengan sebagian pemeriksaan heuristik dilewati. Hasilnya hanya diterima bila sidiknya cocok dengan `StateProof`. Karena itu undo bisa gagal dengan error bertipe, tetapi tidak bisa merusak model.

## Invariant dan pengujian yang relevan untuk kita

- Tidak ada perbaikan geometri diam-diam. Operasi yang menghasilkan topologi invalid selalu ditolak. Pola ini sama dengan prinsip `plan/arsitektur.md` §1.5.
- Deterministik: tidak ada koleksi yang urutannya bergantung pada hash, jadi file `.hew` stabil sampai level byte.
- Contoh property test: `generalized_push_pull_is_watertight_and_invertible_or_untouched` (`tests/op_specs.rs:2969`) dan `push_pull_of_every_result_face_then_undo_is_the_identity` (`tests/face_arrangement_specs.rs:329`). Ada juga fuzz test (`tests/op_fuzz.rs`) dan test determinisme.
- Predikat eksak yang dipakai baru orientasi 2D (gaya Shewchuk, `geom2d.rs:115`). Uji kontak lainnya berbasis jarak metrik terhadap toleransi.

## Temuan penting untuk spike berikutnya

1. **Toleransi absolut nanometer vs koordinat UTM (kriteria #3, Hari 6).**
   - `tol::POINT_MERGE` dan `PLANE_DIST` bernilai `1e-9` m.
   - Pada northing UTM Indonesia (Y ≈ 9.150.000 m), jarak antar-`f64` yang bisa direpresentasikan (ulp) = 2⁻²⁹ ≈ **1,86e-9 m**, lebih besar dari toleransi itu. Pada easting (X ≈ 430.000 m), ulp ≈ 5,8e-11 m, jadi masih aman.
   - **Hipotesis:** push/pull pada geometri yang benar-benar diletakkan di Y ≈ 9,15e6 bisa ditolak atau tidak stabil, karena uji "koinsiden" lebih ketat dari resolusi angkanya. Ini belum diuji.
   - Mitigasi yang sudah ada di rencana kita: geometri disimpan relatif terhadap origin proyek `f64` (ADR-006), sehingga kernel hanya melihat koordinat lokal. Spike Hari 6 harus menguji kedua cara: koordinat absolut dan koordinat lokal + origin.
2. **Kernel WASM (kriteria #5, Hari 11).** Penegasan resmi: kernel berjalan sebagai WASM di desktop. Proses kernel native disebut sebagai "jalan keluar nanti". Crate C/C++ (PROJ, Manifold) tidak bisa langsung masuk ke kernel. Lihat konsekuensi di ADR-000.
3. Boolean memakai implementasi sendiri (`boolean.rs`), bukan Manifold. Bagian ini belum dibaca dalam spike ini.

## Uji pemahaman untuk manusia (kriteria #4)

Kriteria #4 lulus bila Anda bisa menjawab pertanyaan berikut tanpa melihat dokumen ini:
1. Mengapa push/pull yang menembus dinding seberang tidak ditangani oleh `Object::push_pull`?
2. Apa yang terjadi pada Object bila validasi gagal di langkah 5?
3. Kapan undo memakai `unbuild_push_pull`, dan bukan `push_pull(-d)`?
4. Bagaimana undo tahu hasilnya benar?
5. Mengapa `POINT_MERGE = 1e-9` bisa menjadi masalah di Y = 9.150.000?
