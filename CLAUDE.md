# CLAUDE.md

@AGENTS.md

Semua aturan proyek ada di AGENTS.md (diimpor di atas). File ini hanya berisi tambahan khusus Claude Code.

## Khusus Claude Code
- Awal sesi: jalankan Protokol Skill (AGENTS.md §7). Cek skill aktif via daftar skill/plugin; baca SKILL.md `caveman` dan `ponytail`; konfirmasi mode.
- Perintah skill yang mungkin tersedia: `/ponytail`, `/ponytail-review`, `/ponytail-audit`, `/ponytail-debt`, `/ponytail-gain`, `/ponytail-help`, dan varian `caveman-*`. Verifikasi yang benar-benar terpasang dengan `/ponytail-help` dan `/help`; jangan mengasumsikan.
- Pergantian mode lewat perintah kadang tidak tercatat di Claude Code. Setelah ganti mode, tanyakan mode aktif ke diri sendiri dari SKILL.md/keluaran skill, lalu catat di log sesi.
- Skill review tersedia: `/ponytail-review`, `/caveman-review`, dan agen `caveman:cavecrew-reviewer`. Delegasi pekerjaan bulk ke agy lewat agen `antigravity:antigravity-delegate`, lalu verifikasi sendiri hasilnya (jalankan test, cek URL).
- Untuk tugas besar atau multi-file, gunakan plan mode lebih dulu; minta persetujuan manusia sebelum implementasi.
- Gunakan subagent hanya bila diminta manusia atau tugas jelas terpisah (mis. riset library, penulisan test). Beri konteks lengkap karena subagent mulai dari nol.
- Jangan membaca seluruh repo. Mulai dari SPEC.md modul terkait.
- Review silang: setelah selesai, minta manusia menjalankan review oleh Gemini untuk kebenaran geometri pada perubahan di `core-geometry/` dan `inference/`.
