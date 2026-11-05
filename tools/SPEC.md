# SPEC — `tools` (wangungis-tools)

## Tujuan
Lapisan edit tanpa GPU dan tanpa UI. `Scene` membungkus `scene::project::Project` dan menjawab dalam koordinat lokal proyek: snap, pick, gambar, Push/Pull, undo/redo. Seluruh alur bisa diuji otomatis.

## API publik
- `Scene::new(project)`, `project()`, `take_dirty()`: tile yang berubah sejak panggilan terakhir, untuk dibuat ulang mesh-nya oleh renderer.
- `snap(&SnapQuery) -> Option<Hit>`: snap Hew lintas tile. `Hit { tile, snap }`: posisi lokal proyek; `snap.source` dan id sketch berlaku di `tile`.
- `pick_face(&PickRay) -> Option<FaceHit>`: face solid terdekat.
- `draw(tile, plane, segments) -> Drawn`: menggambar segmen sebagai **satu** langkah undo lewat mesin drafting Hew (`Document::draft`), di tile `tile` atau tile milik titik pertama. Semua gambar pada bidang itu menyatu: bentuk di atas face solid membelah face (auto-split, #14), garis yang bertemu gambar lain tersambung dengannya. Tile dibuat bila belum ada. `Drawn { tile, sketch, closed }`: `closed` berarti ada region atau face baru.
- `extrude(tile, sketch, region, distance)`, `push_pull(tile, source, distance)`.
- `movable_at(&Hit) -> Option<Movable>`, `movable_lines(&Movable)`: yang diangkat Move, yaitu seluruh node puncak (objek, grup, instance) atau gambar tersambung pada sketch bebas.
- `movable_center(&Movable)`: pusat batas benda, titik acuan Skala.
- `transform(&Movable, &Transform)`, `copy(&Movable, &step, count)`, `respace_copy(&Movable, &step, count)`: transform dalam koordinat lokal proyek, satu langkah undo. Hasil tetap di tile asal; melewati 1,5 km dari pusat tile atau batas proyek ditolak sebelum dokumen berubah. Pengecualian: gambar yang disalin keluar bidangnya dicatat satu langkah per salinan.
- `erasable_at(&Hit) -> Option<Erasable>`, `erasable_lines`, `erase`: garis gambar (bebas atau pembagi face) hilang sendiri, dan dua face yang dibaginya menyatu; tepi asli solid menghapus seluruh objek puncak. Face dan sudut tidak bisa dihapus.
- `draw_curve(tile, plane, points, closed, center, kind)`: lingkaran, poligon, atau busur sebagai satu kurva Hew dan satu langkah undo.
- `offset(tile, plane, at, distance)`: offset region atau face yang memuat `at`; positif ke luar.
- `undo()`, `redo()`, `can_undo()`, `can_redo()`: mengikuti urutan edit lintas tile.
- `EditError { OutOfTile, OutOfProject, Kernel, Unsupported }`, dengan pesan bahasa Indonesia lewat `Display`.

## Tool (`tool`, `vcb`)
Tool adalah mesin status tanpa GPU dan tanpa UI. `Tool::{select, line, rectangle, circle, polygon, arc, push_pull, offset, move_tool, rotate, scale, eraser}(aperture)`, lalu `handle(&mut scene, input)` dan `preview()`.
- Input: `Hover(ray)`, `Click(ray)`, `Typed(teks)`, `LockAxis(sumbu)`, `Cancel`, `ToggleCopy`.
- `Preview { lines, snap }`: garis overlay dan isyarat snap, dalam koordinat lokal proyek. Digambar UI.
- `busy()`: ada gesture berjalan. `reset()`: kembali ke awal; dipanggil UI setelah undo/redo karena id lama bisa basi.
- `selected()`: hasil pilihan Select (`Target::Solid` atau `Target::Region`).
- `Scene::{face_shape, region_shape, region_count, outline}` memberi bidang dan batas untuk tool.

Perilaku:
- **Bidang gambar** ditentukan klik pertama: face atau region di bawah kursor (juga saat snap jatuh di tepi atau sudutnya), atau bidang horizontal melalui titik snap lain, atau tanah z = 0. Titik snap diproyeksikan persis ke bidang. Klik pertama di tepi atau sudut belum menentukan face mana yang dimaksud, jadi sebelum ada segmen, bidang mengikuti face di bawah kursor yang memuat titik itu.
- **Line:** klik berantai pada bidang yang sama, dengan inference sumbu dari titik sebelumnya. Menutup loop mengakhiri rantai. Esc berhenti, segmen yang sudah jadi tetap ada. Satu segmen = satu langkah undo. Garis baru menyambung gambar lama di bidang yang sama. Gambar di dua tile tidak menyatu (ponytail: tile ditentukan klik pertama).
- **Rectangle:** dua klik, atau klik lalu ketik ukuran. Tanpa inference sumbu dari sudut pertama, karena itu menghapus luasnya. Ukuran ketikan mengikuti kuadran kursor. Luas nol tidak menggambar dan gesture tetap terbuka.
- **Push/Pull:** klik region atau face, lalu klik atau ketik jarak. Region dipilih lewat snap sempit, face lewat pick, supaya klik tepat di tepi tetap kena; yang lebih dekat menang. Pratinjau berupa garis tepi (ponytail: pratinjau solid menyusul).
- **Move:** klik benda, lalu klik tujuan atau ketik jarak searah kursor. `ToggleCopy` membuat langkah berikutnya menyalin, lalu kembali ke memindah (`copying()`). Tepat setelah menyalin, `x5` membuat lima salinan sejarak salinan pertama, dan `/5` membuat lima salinan yang mengisi jarak itu; angka baru menggantikan array sebelumnya, dan semuanya tetap satu langkah undo. Benda yang dipindah tidak bisa snap ke dirinya sendiri. Tanpa snap, titik bergerak di bidang horizontal melalui titik dasar, atau di sumbu terkunci.
- **Circle, Polygon:** klik pusat, lalu klik atau ketik jari-jari. `8s` mengubah jumlah sisi kapan saja; lingkaran minimal 24, bawaan 48; poligon minimal 3, bawaan 6.
- **Arc:** klik dua ujung, lalu klik atau ketik tinggi lengkung, ke sisi kursor.
- **Offset:** klik region atau face, lalu klik atau ketik jarak, ke arah kursor; angka negatif membalik arah.
- **Rotate:** klik benda (titik itu jadi poros), klik arah nol, lalu klik atau ketik sudut dalam derajat. Sumbu Z kecuali dipilih lewat panah. Sudut menempel ke kelipatan 15° bila dalam 1,5°. Menyalin dan array seperti Move, jadi `x5` membuat array melingkar.
- **Scale:** klik benda, lalu gerakkan kursor menjauh atau mendekati pusatnya dan klik, atau ketik faktor. Selalu seragam, terhadap pusat benda. Faktor harus positif.
- **Eraser:** arahkan ke tepi; yang akan hilang menyala oranye, klik menghapusnya sebagai satu langkah undo.
- **Kunci sumbu:** untuk Line, hanya sumbu yang sebidang dengan gambar. Memanggil sumbu yang sama lagi melepas kunci. Pada Line, mengubah kunci melupakan posisi kursor, jadi panjang yang diketik menunggu `Hover` berikutnya dan tidak memakai arah dari sebelum kunci.
- Galat tidak mengubah status tool, jadi pengguna bisa mencoba lagi.

VCB (`vcb::parse`): `5`, `2,5` atau `2.5` (koma adalah desimal), `5;4` atau `5x4`, satuan `mm`/`cm`/`m` (default m), tanda negatif untuk arah berlawanan.

## Invariant (diuji di `tests/scene.rs` dan `tests/tools.rs`)
- Menggerakkan kursor tidak pernah mengedit (property test, semua tool, sinar acak).
- Esc tidak meninggalkan apa pun dan tidak mencatat langkah undo.
- Alur uji asap lewat skrip input: kotak 5×4×3 m ≤ 1 mm, undo, redo, save/open.
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
- Push/Pull di dalam komponen menghasilkan `Unsupported`.
- Tile kosong hasil undo tetap ada di memori untuk redo, tetapi tidak ikut disimpan (`Project::occupied`).

## Non-goals
Renderer, UI, auto-split face, Move/array.
