# 2026-10-29 Sesi 4–6 — dokumen Tahap 1–6

Tujuan: Tahap 1–6 + lampiran. Manusia setujui arah ADR-001..007 (tetap Proposed).

- Selesai: `plan/produk.md` (visi, USP, non-goals, 4 persona, 13 user story, MoSCoW, 7 metrik), `plan/feature-matrix.md` (100 fitur: M56 S26 C16 W2; kolom Hew), `plan/arsitektur.md`, `agents/prompts.md` (8 prompt + usulan AGENTS/CLAUDE/GEMINI), `plan/roadmap.md`, `plan/risiko.md` (16 risiko), `RENCANA.md` (ringkasan, indeks, 14 hari, 8 keputusan, glosarium).
- Pain point agy digabung ke `research/kompetitor.md` §3. Verifikasi: 68/80 URL 200; A2, A3 cocok; A5 dilebih-lebihkan (bukan jitter, tapi geometri hilang/clipping) → dikoreksi.
- Positioning baru: editor koreksi untuk hasil pipeline otomatis (3dfier/roofer) + LOD3.
- Roadmap likely s.d. v0.5: jalur A ±540 j, B ±740 j manusia. Asumsi; koreksi setelah F0.
- Link check: tautan internal 0 rusak; 4 tautan eksternal mati diganti/ditandai; sisanya 403/timeout = blokir bot.
- Tidak dilakukan: `git init`, perubahan AGENTS.md/CLAUDE.md, GEMINI.md (menunggu K4).
- Tangga ponytail: dokumen digabung seminimal mungkin (RENCANA.md hanya indeks + ringkasan; alur agen + prompt satu file).

Langkah berikut: Hari 1 dari "14 hari pertama" (RENCANA.md §3). Review silang Gemini atas ADR + roadmap.

Tambahan (K4 disetujui): AGENTS.md §7 (versi skill terverifikasi; metrik ganti `/ponytail-gain`), §8 langkah 3a (cek Hew/library dulu), §13 branch `spike/…`. CLAUDE.md: skill review + delegasi agy. GEMINI.md dibuat (`@./AGENTS.md`; impor didukung Gemini CLI 0.38.0). AGENTS.md §1/§3 tetap TODO sampai nama + spike.

Tambahan (K1 diputuskan): manusia terima AGPL-3.0-only. ADR-000 #7 lulus; catatan di ADR-005; RENCANA K1 ditandai. Vault: hub Decisions log + status. Berikut: K3 nama, lalu Hari 2.

Tambahan (K3 diputuskan): nama **WangunGIS** (`wangungis` kosong di crates/PyPI/npm, org GitHub kosong). Riset di `research/nama.md`. AGENTS.md §1 diisi. Belum: cek merek PDKI/TMview, rename folder/vault. Berikut: Hari 2.
