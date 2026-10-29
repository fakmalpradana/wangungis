# ADR-001: Bahasa inti — Rust

- Status: Proposed (berlaku untuk jalur A dan B)
- Konteks: satu manusia mereview kode yang sebagian besar ditulis agen. Library wajib (AGENTS.md §5) harus terjangkau.

## Keputusan
Rust untuk semua modul inti. Python untuk plugin dan alur yang butuh IfcOpenShell/GDAL/PDAL penuh.

## Alasan
- Kompiler menangkap kelas bug memori yang paling sulit direview. Dalam studi error kode LLM, 16,7% error kompilasi Rust adalah pelanggaran ownership/lifetime yang di C/C++ menjadi bug diam ([ringkasan](https://codex.danielvaughan.com/2026/08/13/unreliable-in-practice-llm-generated-code-errors-codex-cli-posttooluse-verification-memory-safety-input-validation/); angka belum dicek ke paper asli).
- Library yang dibutuhkan USP tersedia: Manifold (`manifold-csg`), Clipper2/`i_overlay`, `robust`, `proj`, `cjval`/`cjseq`, `tyler`, `ort`, PyO3.
- Jalur A (Hew) sudah Rust.

## Alternatif ditolak
- **C++**: akses langsung CGAL/OpenCascade/IfcOpenShell/OpenUSD, tetapi risiko bug memori pada kode agen lebih tinggi dan beban review lebih besar.
- **Python penuh**: terlalu lambat untuk kernel dan viewport interaktif.

## Konsekuensi
- CMake dan compiler C++ tetap dibutuhkan untuk binding (`manifold-csg`).
- Library C++ besar tanpa binding matang (IfcOpenShell, GDAL penuh) dipakai lewat proses/plugin Python, bukan FFI.
