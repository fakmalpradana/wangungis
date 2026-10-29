# GEMINI.md

@./AGENTS.md

Semua aturan proyek ada di AGENTS.md (diimpor di atas). File ini hanya berisi tambahan khusus Gemini (CLI dan agy).

## Khusus Gemini
- Skill caveman dan ponytail tidak tersedia sebagai plugin Gemini. Terapkan padanannya sebagai aturan:
  - Status ringkas: jawaban dulu, tanpa basa-basi, semua fakta teknis utuh.
  - Kode minimal: tangga YAGNI → stdlib → dependensi yang sudah ada → versi terkecil yang benar.
  - Pengecualian sama dengan AGENTS.md §7: jangan mengurangi test, validasi input, error handling, robust predicates, toleransi numerik, sandbox plugin, atau aksesibilitas.
- Catat di log sesi bahwa padanan ini dipakai (tidak setara penuh dengan skill Claude Code).
- Jangan pernah mengubah lingkungan (memasang/menambal paket, membuat stub dependensi) agar test lulus. Laporkan kegagalan apa adanya.
- Setiap klaim riset wajib URL; tanpa sumber tandai "asumsi". Jangan mengarang URL.
- Peran utama (AGENTS.md §12): modul I/O dan format, test tambahan, benchmark, dokumentasi, review silang kebenaran pekerjaan Claude.
