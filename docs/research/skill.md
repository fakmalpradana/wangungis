# Riset Tahap 0.E — Efektivitas skill caveman dan ponytail

Status: Sesi 3 (2026-10-28). Tautan diakses 2026-10-28.

## Ringkasan

Kedua skill bermanfaat, tetapi jauh lebih kecil dari klaim README-nya. Anggap keduanya sebagai **aturan gaya yang murah**, bukan sumber penghematan besar. Ukur dampaknya sendiri di proyek ini.

## caveman (output ringkas)

| Klaim / temuan | Angka | Sumber |
|---|---|---|
| Klaim README | Rata-rata pengurangan token output ~65% (judul 75%) | [implicator.ai](https://www.implicator.ai/caveman-claude-code-skill-cuts-output-20-your-bill-barely-notices-2/) |
| Uji ulang komunitas | 12–23% pengurangan token output | [implicator.ai](https://www.implicator.ai/caveman-claude-code-skill-cuts-output-20-your-bill-barely-notices-2/) |
| Dibanding instruksi "be concise" biasa | Hanya ~4% lebih baik (uji tiga lengan, Kuba Guzik) | idem |
| Porsi output dari total biaya | 0,6–2,5% → penghematan total tagihan ±1–3% | idem |
| Benchmark plugin ponytail (n=1/sel, 5 tugas kecil) | Caveman −15% token vs baseline; ponytail v3 hampir sama dengan caveman | `~/.claude/plugins/cache/ponytail/ponytail/4.7.0/benchmarks/results/2026-06-12-caveman-vs-ponytail.md` (ditulis pembuat ponytail, bukan independen) |
| Risiko akurasi dari gaya ringkas | Preprint 2026: batasan ringkas justru menaikkan akurasi model besar ~26 poin pada benchmark penalaran. Preprint 2024: CoT dibatasi 100 kata menaikkan akurasi LLaMA2-70b di GSM8K. Keduanya belum peer-review dan khusus tugas penalaran/matematika. | [arXiv 2604.00025](https://arxiv.org/pdf/2604.00025), [arXiv 2407.19825](https://arxiv.org/pdf/2407.19825) |

Catatan: sebagian sumber adalah blog sekunder; tidak ditemukan evaluasi pihak ketiga yang formal.

## ponytail (kode minimal)

| Klaim / temuan | Angka | Sumber |
|---|---|---|
| Klaim pihak ketiga | "memotong 54% kode" | [pasqualepillitteri.it](https://pasqualepillitteri.it/en/news/5720/ponytail-ai-skill-less-code) |
| Benchmark plugin sendiri | ~47 vs ~293 baris kode pada 5 tugas kecil; token total −16% vs baseline | benchmark plugin (lihat atas) |
| Verifikasi independen | Tidak ditemukan | [ringkasan pencarian](https://rajeevpentyala.com/2026/08/09/ponytail-make-your-coding-agent-write-less-code/) |
| Batas yang dijanjikan | Validasi trust boundary, penanganan kehilangan data, keamanan, aksesibilitas tidak dipangkas | [wikidocs](https://wikidocs.net/375905) |

Catatan: benchmark ponytail memakai tugas kecil (email, debounce, csv-sum). Tidak ada data untuk kode geometri, di mana "lebih sedikit kode" bisa berarti robustness hilang. Ini sesuai dengan pengecualian di AGENTS.md §7.

## Ekspektasi realistis untuk proyek ini

1. caveman menghemat ±10–20% token output dan hampir tidak berdampak pada kuota total, karena biaya didominasi input/cache. Manfaat utamanya: log sesi dan komentar review lebih cepat dibaca.
2. ponytail paling berguna di glue UI, wrapper I/O, dan boilerplate. Di `core-geometry/` dan `inference/`, ponytail berisiko memangkas validasi. Pertahankan mode `lite` di sana dan andalkan property test sebagai pagar.
3. Penghematan kuota terbesar justru dari **ukuran konteks**: tugas kecil per sesi, SPEC.md per modul, log sesi, dan menghindari membaca seluruh repo (AGENTS.md §16).

## Metrik yang dicatat sendiri (mengganti `/ponytail-gain`)

`/ponytail-gain` hanya menampilkan median benchmark plugin, bukan ukuran repo ini. Per fase catat:
- Baris kode per modul (`tokei`), rasio test:kode.
- Jumlah dependensi langsung dan transitif (`cargo tree | wc -l`).
- Waktu build bersih dan inkremental di CI per OS.
- Token per sesi dari laporan penggunaan Claude Code (`/caveman-stats` atau laporan bawaan host).
