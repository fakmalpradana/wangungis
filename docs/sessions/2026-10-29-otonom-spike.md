# 2026-10-29 09.14–11.00 — spike otonom (izin manusia: "putuskan sendiri s.d. 11.00")

Tujuan: kerjakan Hari 5–11 tanpa menunggu manusia; aksi publik/akun tetap ditahan.
- S2 #3 (`research/hew-spike-s2-utm.md`): UTM absolut → 3–13% push/pull ditolak, lingkaran r=3 m ditolak saat ekstrusi, render f32 tanpa RTC (kotak 4,244 m tergambar ±5 m). Sweep 36 sudut: 0 tolak s.d. ±2,1e6 m. #3 gagal absolut, lulus hanya dengan origin lokal.
- FPS #2 (`research/hew-spike-fps.md`): kota tiruan 25 ribu × 40 face = 8,6 FPS (2 draw call/Object); 167 ribu Object = crash. Gagal.
- #5 (`research/hew-spike-ext.md`): attribute dictionary simpan CRS/origin + atribut Object (bertahan edit/undo/reload); ekspor CityJSON lolos val3dity. Celah: atribut per face, UI → bergantung #6.
- S3 (`research/spike-s3-manifold.md`): proptest 1.000 lulus lokal (13 face, ≤1e-9 m); UTM pecah.
- S4/CI (PR #2): cjval + val3dity 3 OS hijau, log dicek.
- A2 (`research/spike-a2-native-render.md`): kernel Hew native + wgpu batching = 203 FPS kota, RSS 3,5 GB. Inference: rebuild penuh tiap edit (5,9 dtk) → per tile 7,8 ms, 98% identik dengan global.
- Review silang Gemini PR #3: 12 temuan, semua ditangani (komentar PR). agy pro tier tetap gagal (K8) → `-m "Gemini 4 Argon (High)" --yolo --sandbox` di clone sekali pakai.
- Keputusan agen: issue Hew TIDAK diposting; PR #2/#3 TIDAK di-merge (butuh review manusia, AGENTS §12); config agy tidak diubah.
- Bahan K2: `research/keputusan-k2.md` (rekomendasi A2, bukan keputusan). ADR-001..004 diberi catatan A2, tetap Proposed.
- Belum: #4 (manusia), #6 (posting), Tauri WKWebView/DPR2, UI egui, snapping lintas tile.
- Artefak besar di luar repo: `~/Desktop/3_dummy/hew-spike/app/{public,dist-spike}/*.hew` (±0,5 GB), `~/Desktop/3_dummy/wangungis-spikes/`.

Langkah berikut: manusia jawab #4, setujui posting #6, putuskan aturan tiga jalur + K2; review/merge PR #2, #3.
