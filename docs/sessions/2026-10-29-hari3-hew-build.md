# 2026-10-29 Hari 3 — spike S1-a: build + test Hew (kriteria #1)

Tujuan: clone Hew, build, jalankan gate test-nya, catat waktu.

- PR #1 di-merge ke main (`ece71f7`).
- Clone `hew3d/hew` v1.3.1 (`612c1c1`) ke `~/Desktop/3_dummy/hew-spike` (di luar repo kita, 140 MB).
- Toolchain: rustup pasang Rust 1.97.1 otomatis (pin `rust-toolchain.toml`); wasm-pack 0.15.0 via `scripts/install-wasm-pack.sh` (rilis resmi rustwasm); Node 26.7.0 sudah cocok `.node-version`; pnpm 11.20.0 via corepack.
- Urutan: install-wasm-pack → `pnpm install --frozen-lockfile` → `cargo build --workspace` → `scripts/verify.sh`. Total 819 dtk, exit 0, "verify: all green".
- Hasil: Rust 2.935 lulus / 0 gagal / 3 ignored; Vitest 6.219 lulus (282 file); clippy `-D warnings` + fmt bersih (workspace + shell Tauri). Clone tetap bersih (`git status` kosong).
- Peringatan non-fatal: `buf_redux 0.8.4`, `multipart 0.18.0` "will be rejected by a future version of Rust".
- Tidak dijalankan: `scripts/verify-full.sh` (replay gate + E2E Playwright), `tauri dev`.
- Kriteria #1: **lulus** (< 1 jam kerja). Catatan untuk #4: `crates/` ±279 ribu baris Rust, jadi target "paham alur push/pull dalam 1 hari" harus fokus ke satu jalur kode.

Langkah berikut: Hari 4, baca `docs/dev/ARCHITECTURE.md` + kernel push/pull, lalu tulis `docs/research/hew-kernel.md`.
