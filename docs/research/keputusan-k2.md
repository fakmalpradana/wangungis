# Bahan keputusan K2: jalur A, A2, atau B

Tanggal: 2026-10-29. Ringkasan dari spike Hari 3–11 (rincian di `docs/adr/ADR-000-strategi-hew.md`). Dokumen ini bukan keputusan; keputusannya milik manusia.

## Tiga pilihan
| | **A** — bangun di atas aplikasi Hew | **A2** — pakai crate inti Hew sebagai library, aplikasi native sendiri | **B** — dari nol |
|---|---|---|---|
| Yang dipakai dari Hew | Seluruh aplikasi (kernel WASM, UI React, renderer three.js) | `kernel`, `inference`, `tessellate`, `mesh-export` (±81 ribu baris Rust murni) | Hanya referensi |
| Yang ditulis sendiri | Fitur LOD/geo, lalu perbaikan renderer, indeks, dan origin-shift di kode Hew | UI + tool (egui), renderer wgpu ber-batching, origin proyek, fitur LOD/geo | Semuanya |
| Lisensi | AGPL-3.0-only | AGPL-3.0-only | GPL-3.0-or-later atau AGPL |
| FPS kota 1M face (25 ribu Object) | **8,6** (diukur) | **203** (diukur, prototipe) | belum ada |
| Model 167 ribu Object | tab crash | 232 FPS, RSS 2,7 GB | belum ada |
| Koordinat UTM | gagal tanpa origin lokal (kernel menolak, render salah 0,76 m) | origin lokal sejak awal (ADR-006), terbukti lulus | rancangan sama dengan A2 |
| Jeda snap setelah edit | 3,5–6 detik (perlu perbaikan `inference`) | 7,8 ms dengan tile (prototipe, tanpa mengubah Hew) | — |
| Ketergantungan pada pengelola Hew | Tinggi: renderer, UI, format, indeks | Sedang: hanya untuk atribut per face; tanpa itu, patch kecil di fork | Tidak ada |
| Estimasi v0.1 (**asumsi**) | ±100–160 jam (40 + batching 40–80 + indeks + origin-shift) | ±140–200 jam | ±200 jam |
| Status menurut aturan ADR-000 | Tidak bisa lulus (#2 gagal, #5 bergantung #6) | Aturan tiga jalur masih usulan | Cadangan |

## Rekomendasi agen (bukan keputusan)
**A2**, dengan alasan berikut.
- Dua kelemahan Hew yang terukur, yaitu renderer tanpa batching dan kernel yang terkunci di WASM tanpa origin lokal, berada di lapisan web. Lapisan itu memang akan kita ganti.
- Bagian terbaik Hew, yaitu kernel dengan property test dan undo yang tidak bisa merusak model, bisa dipakai apa adanya. Hal ini sudah dibuktikan secara native oleh 4 prototipe: ekspor CityJSON yang lolos val3dity, render, inference, dan tile.
- Biayanya: seluruh UI/tool ditulis ulang, dan crate Hew harus disinkronkan berkala.
- Biaya sinkronisasi terukur kecil. Hitungan baris `pub fn/struct/enum` di `kernel`, `inference`, dan `tessellate` (heuristik berbasis baris; perubahan signature terhitung hapus + tambah):
  - v1.2.0 → v1.3.1 (2 minggu): **0** item publik dihapus atau diubah, 21 ditambah.
  - v1.0.0 → v1.3.1 (2 bulan): 12 dihapus atau diubah, 194 ditambah.
  - Dalam 30 hari terakhir hanya ada 15 commit di ketiga crate itu. Perubahan API-nya sebagian besar berupa penambahan.

## Yang masih harus Anda putuskan atau kerjakan
1. **#4:** jawab 5 pertanyaan uji di `docs/research/hew-kernel.md`. Bila gagal, kernel Hew terlalu sulit dipahami, dan A2 kehilangan alasannya.
2. **#6:** setujui atau ubah draf issue `docs/research/hew-issue-draft.md`, lalu posting (atas nama Anda).
3. **Aturan keputusan tiga jalur** di ADR-000: setujui atau ubah.
4. **K2:** A, A2, atau B.
