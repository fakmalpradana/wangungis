# 2026-11-25 workflow binary 3 OS

- Tujuan: engineer uji beta tanpa pasang Rust.
- Selesai: `.github/workflows/binaries.yml`. Build release di ubuntu-24.04, macos-15 (arm64), windows-2025. Isi arsip: binary, LICENSE, OFL font. Ditambah SHA-256. Nama berisi 7 huruf commit.
- Pemicu: tag `v*`, manual (workflow_dispatch), PR yang mengubah workflow ini.
- Keputusan: hanya artifact, tanpa GitHub Release, signing, atau notarisasi. Release ikut PR persiapan v0.1.0. Tar untuk Unix karena zip artifact menghapus bit executable.
- Tangga ponytail: tidak ada action pihak ketiga baru, 1 berkas YAML. `--version` tidak ada di app, smoke run dilewati.
- Penulis: Sonnet medium. Review dan penamaan commit: Opus. actionlint bersih.
- Belum: macOS Intel, binary Windows masih butuh VC++ Redistributable (sudah ada di panduan).
- Berikut: unduh artifact Windows, uji bagian A oleh pemilik.
