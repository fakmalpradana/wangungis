# SPEC — `scene` (wangungis-scene)

## Tujuan
Menyimpan dan membaca **origin proyek** (kode EPSG + titik origin `f64`) di dokumen Hew, dan menetapkan batas koordinat yang aman untuk kernel Hew (ADR-004, ADR-006).

## API publik
- `NS = "wangungis"`: namespace attribute dictionary Hew untuk semua data WangunGIS.
- `PROJECT_BOUND_M = 50_000.0`: batas |koordinat| per sumbu relatif terhadap origin proyek, yaitu luas satu proyek (satu kota).
- `KERNEL_FRAME_BOUND_M = 1_500.0`: batas jarak horizontal geometri dari origin `Document` Hew tempat geometri itu berada.
- `ProjectOrigin { epsg, origin }` dengan fungsi:
  - `read(&Document) -> Result<Option<_>, OriginError>`
  - `write(&mut Document)`
  - `to_local(world) -> Result<Point3, _>`
  - `to_world(local)`
- `check_local([f64; 3])`, `OriginError { Malformed, OutOfBound, Attr }`.

## Invariant
- `write` lalu save/load menghasilkan nilai `read` yang **bit-identik**.
- Atribut yang ada tetapi rusak menghasilkan `Err(Malformed)`, bukan nilai default.
- `to_local` menolak titik non-finite atau di luar ±`PROJECT_BOUND_M`.
- `to_world(to_local(w))` sama dengan `w` dalam batas ≤ 1 ulp per sumbu.

## Batas kernel (K6)
Diuji di `tests/kernel_bound.rs`:
- Property test di dalam radius `KERNEL_FRAME_BOUND_M`, 256 kasus per jalan CI, dan sudah diuji lokal sampai 5.000 kasus:
  - push/pull pada bidang miring 15–60°
  - lingkaran 48 segmen
  - boolean pocket pada kotak yang diputar

  Kernel tidak boleh menolak satu kasus pun, dan galatnya harus ≤ 1 mm.
- Uji karakterisasi (hanya di release): boolean pada kotak yang diputar **ditolak** pada 10 km. Bila Hew memperbaikinya, test ini gagal sebagai tanda untuk meninjau ulang batas.

Konsekuensinya, satu `Document` Hew tidak bisa memuat satu kota. Proyek dipecah menjadi tile kernel dengan frame di pusat tile (ADR-004).

## Non-goals
- Transformasi CRS (PROJ): menyusul di v0.2, di `io`.
- Manajemen tile kernel: menyusul di PR berikutnya, dibangun di atas crate ini.
