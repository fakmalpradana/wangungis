# 2026-11-05 point cloud: render, LOD, snap, potong
Tujuan: AQ_27.las (26,5 juta titik, 902 MB) tampil ringan dan jadi acuan menggambar, dengan ukuran titik dan potong kotak volume. Permintaan pemilik.
Selesai: `CloudSet` (LOD awalan, thread baca, anggaran), pipeline titik di `render`, snap titik di `Scene::snap`, `Tool::cloud_slice`, panel, berkas pendamping.
Keputusan: rencana membagi render dan potong jadi PR 13 dan 14; digabung satu PR karena panel dan shader saling terkait. Pegangan pakai klik-gerak-klik seperti Push/Pull.
Keputusan: origin proyek hanya diambil dari awan pada proyek baru. Hew mencatat `attr_set` di undo, jadi mengubahnya di proyek berisi merusak riwayat.
Terukur: lihat `pointcloud/SPEC.md`. Semua target lulus di Mac ini. Disk dingin dan GPU lain belum diukur.
Bug tertangkap test: titik tetap tampil setelah awan terakhir dilepas; membuka proyek menandai ada perubahan; anggaran memori tidak menghitung bacaan yang berjalan; lantai per chunk melewati anggaran gambar.
Batas: klik menembus atap ke tanah butuh snap titik dimatikan. Kotak sejajar sumbu. Solid menutupi titik di dalamnya.
Review: Codex memberi 5 temuan, semua valid dan diperbaiki: lantai LOD melewati anggaran, kotak dan frame lama setelah awan terakhir dilepas, gagal tulis berkas pendamping tetap dianggap tersimpan, autosave tanpa awan, tool potong menandai perubahan tanpa perubahan. agy: bersih.
Jendela asli: menu, jendela Format, dan panel OS terbukti. Bug ketemu: membuka lewat argumen sebelum `cache_dir` terisi. Memilih file di panel OS tidak boleh saya lakukan. Layar terkunci sebelum uji AQ_27 di jendela selesai.
Berikut: pemilik uji langkah 30-34, izin push.
