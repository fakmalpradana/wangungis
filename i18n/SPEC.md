# SPEC — `i18n` (wangungis-i18n)

## Tujuan
Teks layar dalam bahasa pilihan pengguna, tanpa dependensi baru selain `serde_json`. Satu bahasa aktif untuk seluruh proses. Tanpa mesin jamak, ICU, atau Fluent.

## API publik
- `Lang`: Id, En, ZhHans, ZhHant, Ja, Ko, Ar, Es, PtBr, Fr, De, Ru, Tr, Vi, Th, Hi, dalam urutan itu. `Lang::ALL`, `code()` (`id`, `en`, `zh-Hans`, `zh-Hant`, `ja`, `ko`, `ar`, `es`, `pt-BR`, `fr`, `de`, `ru`, `tr`, `vi`, `th`, `hi`), `from_code()`, `native_name()`, `rtl()` (hanya Ar), `machine_draft()` (semua kecuali Id dan En).
- `set(lang)`, `current()`: satu `AtomicU8`, bawaan Id.
- `t(kunci) -> &'static str`: teks bahasa aktif.
- `tf(kunci, &[&dyn Display]) -> String`: `t` dengan `{0}`, `{1}`, ... diganti menurut urutan argumen. Isi argumen tidak dibaca ulang sebagai penanda. Penanda tanpa argumen dibiarkan.

## Katalog
- `locales/<kode>.json`: objek datar kunci → teks, dimuat dengan `include_str!` dan di-parse sekali per bahasa. Teksnya di-leak, karena katalog hidup selama program.
- Saat ini hanya `id.json` (teks Indonesia apa adanya, sumber kebenaran urutan kunci) dan `en.json`. Empat belas bahasa lain jatuh ke fallback; menambah bahasa = menambah berkas dan satu baris di `Lang::source`.
- Fallback: bahasa aktif, lalu `en`, lalu `id`, lalu kunci itu sendiri.

## Font
Glyph untuk semua bahasa datang dari font Noto yang dibundel di crate `ui` (ADR-010); crate ini tidak memuat font. Setiap karakter katalog dan `native_name()` harus punya glyph, dan tes di `ui/src/fonts.rs` memeriksanya terhadap semua berkas di `locales/`.

## Nama kunci
Bertitik, huruf kecil, bahasa Inggris, dikelompokkan menurut tempat: `menu.*`, `toolbar.*`, `tool.hint.*`, `tool.lock.*`, `status.*`, `panel.*`, `dialog.*`, `error.*`, `cloud.*`, `view.*`, `snap.*`, `paint.*`, `outline.*`, `format.*`. Nama stabil, bukan teksnya. Tidak ada kunci yang dirakit dengan `format!`: setiap kunci muncul utuh sebagai literal di kode. Penanda memakai nomor `{0}`.

## Yang tidak diterjemahkan
Log dan teks debug, pesan panic, ekstensi berkas, kode EPSG, satuan (`m`, `mm`, `ft`, `ms/frame`), teks galat dari Hew (`kernel`, `inference`) dan galat sistem operasi: ditambahkan apa adanya ke pesan yang diterjemahkan. Teks `Debug` dari galat internal (`ProjectError`, `ExportError`, dan sejenisnya) juga apa adanya.

## Tes
1. Setiap berkas di `locales/` punya himpunan kunci yang sama dengan `id.json`.
2. Himpunan penanda `{n}` tiap kunci sama di semua katalog.
3. Tidak ada nilai kosong.
4. Setiap literal `t("..")`/`tf("..")` di seluruh workspace ada di `id.json`; setiap literal bergaya kunci (mis. yang lewat `Unsupported(..)` atau tabel label) ada di `id.json`; setiap kunci di `id.json` dipakai di suatu tempat.
5. `t`, `tf`, urutan fallback, dan `Lang::from_code(code())`.
6. `ui/src/smoke.rs`: alur persegi + Push/Pull dengan bahasa Inggris, memastikan baris petunjuk dan pesan galat berbahasa Inggris.
Tes yang mengubah bahasa global memegang kunci (`LANG_LOCK` di crate ini, `GPU_LOCK` di `ui`) dan mengembalikan Id.

## Non-goals
- Tata letak kanan-ke-kiri dan bidi (`rtl()` baru informasi).
- Bentuk jamak dan gender.
- Format angka, tanggal, dan desimal menurut lokal (angka tetap dengan titik desimal).
- Mengganti bahasa tanpa membuat ulang teks yang sudah tersimpan (mis. nama cat yang terpilih, kunci mengunci sudut pada garis yang sedang berjalan).
