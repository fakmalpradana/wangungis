# 2026-11-25 persiapan rilis v0.1.0

- Tujuan: repo siap di-tag v0.1.0-rc.1 lalu v0.1.0.
- Selesai: versi workspace 0.1.0 (+Cargo.lock), README.md (Inggris), CHANGELOG.md, job `release` di `binaries.yml`.
- Job `release`: hanya pada tag `v*`, buat DRAFT GitHub Release berisi arsip 3 OS + sha256, catatan dari CHANGELOG. Tag `-rc` diberi `--prerelease`. Pemilik publikasi manual.
- Keputusan: README Inggris karena repo publik; dokumen pengguna tetap Indonesia, ditandai di README.
- Klaim fitur README dicek ke `docs/user/uji-asap.md`.
- Penulis: Sonnet medium. Review: Opus + Gemini.
- Belum: job release belum pernah jalan (hanya jalan saat tag). Uji pertama = tag rc.1.
- Berikut: merge, tag `v0.1.0-rc.1` (izin pemilik), uji binary Windows/Linux, lalu `v0.1.0`.
