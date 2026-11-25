# CLAUDE.md

@AGENTS.md

Semua aturan proyek ada di AGENTS.md (diimpor di atas). File ini hanya berisi tambahan khusus Claude Code.

## Khusus Claude Code
- Awal sesi: jalankan Protokol Skill (AGENTS.md §7). Cek skill aktif via daftar skill/plugin; baca SKILL.md `caveman` dan `ponytail`; konfirmasi mode.
- Perintah skill yang mungkin tersedia: `/ponytail`, `/ponytail-review`, `/ponytail-audit`, `/ponytail-debt`, `/ponytail-gain`, `/ponytail-help`, dan varian `caveman-*`. Verifikasi yang benar-benar terpasang dengan `/ponytail-help` dan `/help`; jangan mengasumsikan.
- Pergantian mode lewat perintah kadang tidak tercatat di Claude Code. Setelah ganti mode, tanyakan mode aktif ke diri sendiri dari SKILL.md/keluaran skill, lalu catat di log sesi.
- Skill review tersedia: `/ponytail-review`, `/caveman-review`, dan agen `caveman:cavecrew-reviewer`. Delegasi pekerjaan bulk ke agy lewat agen `antigravity:antigravity-delegate`, lalu verifikasi sendiri hasilnya (jalankan test, cek URL).
- Untuk tugas besar atau multi-file, gunakan plan mode lebih dulu; minta persetujuan manusia sebelum implementasi.
- Subagent: untuk coding dan tugas repetitif (delegasi Sonnet di bawah), riset library, atau penulisan test. Beri konteks lengkap karena subagent mulai dari nol.
- Jangan membaca seluruh repo. Mulai dari SPEC.md modul terkait.
- Delegasi coding: `Agent` dengan `model: "sonnet"`, `effort: "medium"`, brief lengkap (SPEC, berkas, test, definisi selesai), larang tulis git. Opus membaca diff, menjalankan gerbang (AGENTS.md §9), commit, dan membuka PR.
- Review silang per PR dijalankan Claude sendiri lewat agy, read-only: `agy-delegate --model "Gemini 4 Argon (High)" --timeout 20m --dir <folder kosong> - < prompt`, prompt diawali "Do not run any command or tool" lalu diff atau berkasnya. Rekonsiliasi temuan terhadap kode; Claude yang memutuskan.
- CI: jangan polling. Cek sekali dengan `gh pr view <n> --json statusCheckRollup`; bila belum selesai, laporkan dan tunggu pemilik.
- Uji di jendela asli macOS: bundle uji di scratchpad dengan HOME sementara, bukan `~/.wangungis` pemilik. Jangan sentuh panel Open/Save OS.
