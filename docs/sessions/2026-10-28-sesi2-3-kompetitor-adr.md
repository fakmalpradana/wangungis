# 2026-10-28 Sesi 2–3 — kompetitor, skill, sintesis, ADR

Tujuan: Tahap 0.A/B/E/F + draf ADR. Manusia pilih jalur C (spike Hew).

- `docs/research/kompetitor.md`: matriks 66 fitur × 11 tool (SU, BL, RH, FC, RV, BB, CE, HO, S3, PL, Hew). Blok LOD/geospasial (baris 37–52) bersumber; sisanya asumsi tingkat sedang.
- Celah: UX gambar-tangan + semantik CityGML + val3dity + CRS/UTM dalam satu tool open source. Tidak ada yang mengisi.
- `docs/research/skill.md`: caveman nyata 12–23% output (klaim 65–75%); ~4% di atas "be concise". ponytail tanpa verifikasi independen. Metrik ganti `/ponytail-gain`: tokei, cargo tree, waktu build CI.
- `docs/research/ringkasan.md`: ringkasan eksekutif, tabel ADR, risiko, definisi setara/unggul.
- `docs/adr/ADR-000..007`. ADR-000 Accepted (spike, 7 kriteria). Lainnya Proposed, bercabang A/B.
- Pain point forum (0.B) didelegasikan ke agy → `_raw-painpoints.md`. Percobaan pertama gagal (model tier pro tidak tersedia; config `tier_pro` agy perlu diganti), retry berjalan. Belum digabung.
- Gagal/batas: sel matriks umum belum dicek satu per satu; CityEngine CityGML tidak ada di daftar format resmi (bukan bukti mutlak).

Langkah berikut: manusia setujui arah ADR-001..007. Gabung pain point. Lalu Sesi 4 (Tahap 1–2).
