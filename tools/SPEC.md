# SPEC — `tools` (wangungis-tools)

## Tujuan
Lapisan edit tanpa GPU dan tanpa UI. `Scene` membungkus `scene::project::Project` dan menjawab dalam koordinat lokal proyek: snap, pick, gambar, Push/Pull, undo/redo. Seluruh alur bisa diuji otomatis.

## API publik
- `Scene::new(project)`, `project()`, `take_dirty()`: tile yang berubah sejak panggilan terakhir, untuk dibuat ulang mesh-nya oleh renderer.
- `snap(&SnapQuery) -> Option<Hit>`: snap Hew lintas tile. `Hit { tile, snap }`: posisi lokal proyek; `snap.source` dan id sketch berlaku di `tile`.
- `pick_face(&PickRay) -> Option<FaceHit>`: face solid terdekat.
- `draw(target, plane, segments)`: menggambar segmen sebagai **satu** langkah undo, pada sketch `target` atau sketch baru di tile milik titik pertama. Tile dibuat bila belum ada.
- `extrude(tile, sketch, region, distance)`, `push_pull(tile, source, distance)`.
- `undo()`, `redo()`, `can_undo()`, `can_redo()`: mengikuti urutan edit lintas tile.
- `EditError { OutOfTile, OutOfProject, Kernel, Unsupported }`, dengan pesan bahasa Indonesia lewat `Display`.

## Invariant (diuji di `tests/scene.rs`)
- Edit yang ditolak tidak mengubah proyek dan tidak meninggalkan langkah undo: batch gagal (diuji dulu pada salinan sketch), segmen di luar tile/proyek, Push/Pull yang melewati `KERNEL_FRAME_BOUND_M`.
- Edit yang ditolak setelah sempat diterapkan kernel juga membersihkan redo proyek, supaya tidak bisa kembali lewat redo.
- Undo hanya bisa mencapai edit pengguna, bukan metadata tile.
- Kotak di mana pun dalam ±5 km berada ≤ 1 mm dari yang diminta, tetap solid (`health_report`), dan bertahan melewati undo, redo, dan save/open (property test).
- Snap benar melintasi batas tile; titik di belakang face di tile lain tidak menang, walau `Endpoint` berperingkat lebih tinggi dari `OnFace`.

## Indeks inference
`InferenceScene` Hew bekerja per `Document` (id objek hanya unik di sana) dan dibangun ulang seluruhnya setelah setiap perubahan. Karena itu setiap tile dipecah menjadi sel `CELL_M` = 200 m:
- Sel menyimpan node dan sketch menurut pusat bounds-nya, plus bounds gabungan untuk menolak sinar secara murah.
- Indeks sel dibuat saat sinar pertama kali lewat di dekatnya. Edit membuang sel seluruh tile, lalu hanya sel yang dilewati sinar yang dibangun lagi.
- Scene `base` tanpa geometri menjawab snap arah murni (sumbu proyek, kunci sumbu).
- Hasil digabung menurut `SnapKind::rank_group`, lalu sudut berbobot ke sinar (dengan toleransi seri Hew), lalu jenis, lalu kedalaman. Jawaban "garis kunci kosong" berperingkat terakhir.
- Oklusi lintas sel: sinar dari mata ke titik snap harus bebas dari face solid di sel mana pun (seperti `is_occluded` Hew), termasuk snap sumbu proyek. Hanya garis kunci kosong yang dikecualikan. Region sketch tidak menutup lintas sel.
- Ini mendekati, tetapi tidak identik dengan, satu scene global: aturan "reach never steals" Hew tidak terlihat dari luar (`docs/research/spike-a2-native-render.md`).

Ukuran nyata `city25k.hew` (24.964 objek, Apple M4, release): snap pertama 51 ms (5,1 dtk bila satu indeks per tile); snap setelah edit 42 ms (324 ms); snap berikutnya ±80 µs; dari ketinggian 5 km, kursor menyapu kota ±78 ms per hover pada kali pertama dan ±18 ms setelahnya; draw/extrude/undo < 1 ms.

## Batas sengaja (ponytail)
- Objek top-level yang disembunyikan lewat tag tetap bisa di-snap (Hew tidak membuka statusnya); user-hidden tidak, begitu juga anggota definisi yang tersembunyi.
- Push/Pull di dalam komponen menghasilkan `Unsupported`.
- Tile kosong hasil undo tetap ada di proyek.

## Non-goals
Tool dan VCB (PR berikutnya di crate ini), renderer, UI.
