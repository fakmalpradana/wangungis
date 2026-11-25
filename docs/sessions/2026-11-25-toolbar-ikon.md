# Toolbar ikon

Tujuan: set ikon yang disetujui pemilik (17 tool, 9 aksi, 6 tampilan) dipasang di toolbar.
Selesai: `ui/src/icons.rs`, parser path SVG kecil (M L H V C S A Z) ke `egui::Shape`, tanpa dependensi. Toolbar jadi tombol ikon 30 px dengan tooltip dan label aksesibilitas.
Pembagian kerja: kode oleh Sonnet effort medium, review, gate, uji jendela, dan commit oleh Opus.
Keputusan: Belakang dan Kiri memakai kubus yang sama dengan face tersembunyi bergaris putus. Ujung garis bulat ditiru dengan lingkaran kecil, karena egui tidak punya line cap.
Keputusan pemilik: tiga assertion tinggi di `smoke::push_pull_two_solids_through_the_app` memakai toleransi 1e-6 m. Lebar toolbar menggeser viewport, sehingga koordinat klik ikut berubah dan normal face Hew membawa selisih 3,6e-15. Janji tinggi persis pada snap tetap diuji di `tools/tests`.
Terlihat di jendela asli (tema terang): semua ikon tampil. Tint face pada ikon tampilan tipis di ukuran 30 px.
Belum: tema gelap dan skala 2,0 belum dilihat langsung.
Langkah berikut: PR B, inti i18n.
