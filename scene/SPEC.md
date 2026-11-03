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
- `check_local([f64; 3])`: cek tingkat proyek (±`PROJECT_BOUND_M`). Cek ini **tidak** menjamin titik aman untuk satu `Document`.
- `OriginError { Malformed, OutOfBound, Attr }`.

## Invariant
- `write` lalu save/load menghasilkan nilai `read` yang **bit-identik**.
- Atribut yang ada tetapi rusak, atau hanya sebagian (`crs` tanpa `origin`), menghasilkan `Err(Malformed)`, bukan nilai default.
- `to_local` menolak titik non-finite atau di luar ±`PROJECT_BOUND_M`.
- `to_world(to_local(w))` sama dengan `w` dalam batas ≤ 1 ulp dari max(|w|, |origin|) per sumbu (property test).
- `epsg` harus > 0; `"EPSG:<digit>"` saja, tanpa tanda.

## Batas kernel (K6)
Diuji di `tests/kernel_bound.rs`:
- Property test di dalam radius `KERNEL_FRAME_BOUND_M`, 256 kasus per jalan CI, dan sudah diuji lokal sampai 5.000 kasus:
  - push/pull pada bidang miring 15–60°
  - lingkaran 48 segmen
  - boolean pocket pada kotak yang diputar

  Kernel tidak boleh menolak satu kasus pun, dan galatnya harus ≤ 1 mm.
- Uji karakterisasi (hanya di release): boolean pada kotak yang diputar **ditolak** pada 10 km. Bila Hew memperbaikinya, test ini gagal sebagai tanda untuk meninjau ulang batas.

Konsekuensinya, satu `Document` Hew tidak bisa memuat satu kota. Proyek dipecah menjadi tile kernel dengan frame di pusat tile (ADR-004).

## Tile kernel dan proyek (`project`)
Satu kota = `Project { origin, tiles }`. Satu tile = satu `Document` Hew, berukuran `TILE_SIZE_M` = 1.000 m.
- `TileKey { i, j }`: `TileKey::of([x, y])` membulatkan koordinat lokal proyek ke pusat tile terdekat. `center()` = `(i·1000, j·1000, 0)`. Tile (0, 0) berpusat di origin proyek, sehingga `.hew` biasa = proyek satu tile.
- `Project::from_document(doc)`:
  - Bila geometri ≤ `KERNEL_FRAME_BOUND_M`, dokumen tetap satu tile dan hanya ditambah metadata tile.
  - Bila lebih jauh, setiap node top-level dipindah ke tile milik jangkarnya (pusat bbox XY vertexnya, termasuk geometri definisi instance).
  - Pemindahan memakai `extract_item` + `insert_document` Hew sebagai translasi murni: axes gambar dipasang sementara di pusat tile, sehingga Hew tidak memakai re-origin bottom-center-nya. Atribut, tag, status user-hidden, grup, dan instance ikut terbawa. Axes asli dikembalikan.
  - Dokumen hasil pecah di-round-trip lewat save/load, sehingga langkah undo dari proses pecah tidak tersisa.
  - Setiap vertex (x, y, z) wajib berada di dalam ±`PROJECT_BOUND_M`; bila tidak, hasilnya `Err`.
  - Tile (0, 0) memakai dokumen sumber itu sendiri, sehingga sketch world, anotasi, guide, scene, dan style tetap ada (tidak digeser). Anotasi yang menempel pada node yang dipindah ikut terlepas; definisi komponen yang hanya dipakai node yang dipindah tetap tersimpan di tile (0, 0).
- `Project::open(bytes)`: membuka wadah proyek, atau `.hew` biasa lewat `from_document`.
- `Project::new(origin)`: proyek kosong berisi tile (0, 0).
- `Project::tile_mut(key)`: dokumen tile, dibuat kosong bila belum ada. Pusat tile wajib di dalam batas proyek.
- `node_visible(doc, node)`: node dan grup di sekitarnya tidak user-hidden dan tidak pada tag tersembunyi. Dipakai `render` dan `tools`, sehingga yang tidak terlihat tidak bisa di-snap.
- `Project::occupied()`: tile yang berisi. Tile kosong sisa undo tetap di memori untuk redo, tetapi tidak disimpan dan tidak dihitung `single()`.
- `node_points(doc, node)`: semua vertex world di bawah node, termasuk geometri definisi instance.
- `Project::is_container(bytes)`: `true` bila bytes berupa wadah proyek, bukan `.hew` biasa.
- `Project::save()`: proyek satu tile disimpan sebagai `.hew` biasa. Selain itu disimpan sebagai wadah zip byte-stable berisi `wangungis-project.json` + `tiles/<i>_<j>.hew`.
- `horizontal_reach(doc)`: jarak horizontal vertex world terjauh (objek dan geometri instance) dari origin dokumen.
- `ProjectError { Origin, Kernel, Container, TooFar }`.

Invariant (diuji di `tests/project.rs`):
- Setelah dipecah, setiap objek muncul tepat satu kali, di tile milik jangkarnya, dengan posisi world bergeser ≤ 1 mm dan atribut tetap (property test, kotak acak ±5 km).
- Setiap tile memiliki jangkauan ≤ `KERNEL_FRAME_BOUND_M`. Objek yang lebih panjang dari ±1 km menghasilkan `Err(TooFar)`.
- save → open → save menghasilkan byte identik.
- Setiap tile menyimpan key-nya (`wangungis.tile`) dan origin miliknya sendiri (origin proyek + pusat tile). Satu tile yang dibuka sendirian, juga di Hew, tetap bergeoreferensi benar.
- Wadah yang tidak cocok dengan yang ditulis crate ini ditolak, bukan ditebak. Contohnya: versi atau ukuran tile lain, origin tile ≠ manifest (> 1 mm), key ganda, tile hilang, entri tambahan, wadah berisi satu tile (0, 0) yang semestinya `.hew` biasa, atau nama entri salah. Total byte yang dibaca dari semua entri tidak boleh melebihi ukuran wadah.

Ukuran nyata: `city25k.hew` (24.964 objek, jangkauan 2,7 km) dipecah dalam 4,1 dtk menjadi 9 tile. Open 1,1 dtk, dan ukuran file sama (139 MB). Diukur di Apple M4, release.

Batas sengaja (ponytail):
- Semua tile dimuat sekaligus.
- Objek yang melewati batas tile setelah diedit belum dipindah otomatis (v0.1, bersama tool Move).

## Non-goals
- Transformasi CRS (PROJ): menyusul di v0.2, di `io`.
