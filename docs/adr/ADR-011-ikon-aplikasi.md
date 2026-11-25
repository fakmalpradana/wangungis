# ADR-011: Ikon aplikasi

- Status: **Diterima** (pemilik menyetujui pemasangan ikon ke jendela dan biner, termasuk dependensi build Windows, 2026-11-25)

## Konteks
Pemilik meminta ikon aplikasi tampil di jendela, taskbar atau Dock, dan di berkas biner. Ikon sudah ada di `ui/assets/`: `icon.svg` (sumber), `icon-1024.png`, `icon-256.png`, `icon.ico` (16 sampai 256 px), dan `icon.icns`.

## Keputusan
- **Ikon saat jalan:** `ui/src/main.rs` memasang `icon-256.png` ke `ViewportBuilder::with_icon`. PNG ditanam dengan `include_bytes!` dan didecode oleh `eframe::icon_data::from_png_bytes`. eframe sudah membawa crate `image`, jadi tidak ada dependensi baru untuk ini. Test memastikan ukuran 256x256, panjang RGBA, sudut transparan, dan bagian tengah buram.
- **Ikon di `.exe` Windows:** `ui/build.rs` memakai `winresource` untuk menanam `assets/icon.ico` sebagai resource. `winresource` hanya build-dependency pada `cfg(windows)`, dan kodenya juga dijaga `#[cfg(windows)]`, sehingga build di macOS dan Linux tidak menyentuhnya.

## Lisensi dan ukuran
- `winresource` 0.1.34: MIT. Dependensinya: `toml` dan `version_check`.
- Turunannya (semua dari `cargo metadata`): `toml` 1.2.3 (MIT OR Apache-2.0), `serde_spanned` 1.1.3 (MIT OR Apache-2.0), `toml_datetime` 1.1.3 (MIT OR Apache-2.0), `toml_parser` 1.1.7 (MIT OR Apache-2.0), `toml_writer` 1.2.0 (MIT OR Apache-2.0), `winnow` 1.0.4 (MIT), `version_check` 0.9.5 (MIT/Apache-2.0). `indexmap` dan `serde_core` sudah ada di pohon.
- Semua kompatibel dengan AGPL-3.0-only. Semuanya hanya crate waktu build; tidak masuk ke biner. `Cargo.lock` mencatat `winresource`, `toml`, `serde_spanned`, dan `toml_writer` sebagai paket baru, serta menaikkan `toml_datetime` dan `toml_parser` yang sudah ada.

## Alternatif ditolak
- **`embed-resource`** (3.0.12, MIT): crate serupa untuk menanam resource Windows. Tidak dipilih karena `winresource` sudah cukup untuk satu ikon (`set_icon`); perbedaan teknis lain tidak diverifikasi.
- **Memanggil `rc.exe` sendiri**: butuh Windows SDK di PATH dan logika pencarian alat di `build.rs`, yang justru ditangani crate di atas.

## Konsekuensi
- Build Windows menjalankan `build.rs` yang menanam ikon. Ini belum dijalankan di Mac pengembang; CI Windows yang memverifikasinya.
- Bundel `.app` macOS dengan `icon.icns` dan berkas `.desktop` Linux belum dibuat. Ikon saat jalan menutupi jendela dan taskbar di Windows dan Linux, serta Dock di macOS (eframe memanggil `setApplicationIconImage`), tetapi ikon berkas biner di Finder dan file manager Linux belum ada.
- Menambah dependensi adalah checkpoint manusia (AGENTS.md §14); pemilik sudah menyetujui.
