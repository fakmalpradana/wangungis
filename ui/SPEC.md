# SPEC — `ui` (binary `wangungis`)

## Tujuan
Shell aplikasi desktop F0. Membuka `.hew` atau proyek `.wangun`, menampilkannya dengan renderer `render`, dan menyediakan navigasi orbit/pan/zoom (#31) serta zoom extents (#32).

## Perilaku
- `wangungis [file.hew | file.wangun]`: membuka file dari argumen. Bisa juga lewat kotak path + tombol **Buka** (atau Enter). Tanpa argumen: proyek baru kosong.
- File dibuka lewat `scene::project::Project::open`. Setiap tile kernel menjadi satu tile render di pusatnya. `.hew` yang geometrinya melewati 1,5 km langsung dipecah menjadi tile 1 km di memori.
- **Tool** (logika di crate `tools`): Pilih (Spasi), Garis (L), Persegi (R), Lingkaran (C), Poligon (G), Busur (A), Push/Pull (P), Offset (F), Pindah (M), Putar (Q), Skala (S), Hapus (E). Klik kiri milik tool. Saat Pindah atau Putar, C atau menekan Ctrl beralih ke menyalin; status bar menulis "Menyalin".
- **Nilai:** angka yang diketik langsung masuk tanpa perlu fokus, ditampilkan di status bar, Enter mengirim, Backspace menghapus, Esc membatalkan. Huruf satuan (`m`, `c`) hanya dihitung setelah ada angka. `x`, `*`, dan `/` boleh mengawali nilai, untuk jumlah array. Klik kiri pada tool membuang nilai yang sedang diketik.
- **Kunci sumbu:** panah kanan = X, kiri = Y, atas = Z. Pratinjau langsung mengikuti tanpa perlu menggerakkan mouse.
- **Kursor diam:** setiap input selain hover membuat tool menerima hover lagi di piksel yang sama, sehingga pratinjau dan penanda snap tetap benar. Tombol navigasi yang ditahan tanpa gerakan tidak meminta repaint.
- **Keyboard dan kotak path:** saat kotak path fokus, tombol masuk ke kotak, kecuali Cmd/Ctrl+S yang tetap menyimpan. Esc yang keluar dari kotak tidak ikut membatalkan gesture. Tab dari viewport tidak berpindah ke kotak atau tombol, karena semua tombol tanpa modifier milik tool dan Enter di kotak mulai membuka path itu. Akibatnya kotak path hanya bisa dicapai dengan klik.
- **Undo/Redo:** Cmd/Ctrl+Z, Shift+Cmd/Ctrl+Z, atau tombol. Gesture yang sedang berjalan dibatalkan, karena id-nya bisa basi.
- **Navigasi:** seret tengah = orbit, Shift+seret tengah = pan. Di trackpad: Alt+seret kiri = orbit, Shift+Alt+seret kiri = pan. Scroll = zoom. **Zoom extents** memasukkan seluruh model ke layar.
- **Overlay:** sumbu proyek (merah X, hijau Y, biru Z), pratinjau tool (oranye), dan penanda snap (hijau titik ujung, sian titik tengah, merah tepi, biru face).
- **Simpan** (Cmd/Ctrl+S): proyek satu tile disimpan sebagai `.hew`, selebihnya `.wangun`. File hanya ditimpa bila jenisnya sama, jadi `.hew` yang dipecah atau bertambah tile disimpan di sebelahnya (`kota.hew` menjadi `kota.wangun`; nama lain mendapat tambahan `.wangun`). Path di kotak atas menentukan tujuan, jadi nama baru berarti Save As; kotak kosong berarti file yang sedang dibuka. File lain yang sudah ada tidak pernah ditimpa: Simpan menolak dan meminta nama lain. Penulisan lewat file sementara, sehingga gagal tulis tidak merusak file lama.
- **Baru:** proyek kosong dan kotak path dikosongkan.
- **Perubahan belum disimpan:** Baru, Buka, dan menutup jendela menanyakan Simpan, Buang, atau Batal lebih dulu.
- **Autosave** (#100): selama ada perubahan belum disimpan, setiap 60 detik saat tool tidak sedang berjalan, proyek ditulis ke `~/.wangungis/autosave.bin`. File dihapus setelah Simpan atau setelah pengguna memilih Buang. File yang masih ada saat aplikasi dibuka berarti sesi sebelumnya tidak berakhir normal, sehingga aplikasi menawarkan Pulihkan atau Buang. Satu file per pengguna: dua jendela saling menimpa (ponytail).
- Status bar: nilai yang diketik, jenis snap, galat, nama file, jumlah tile/objek/segitiga, EPSG + origin, status simpan, waktu per frame.
- Galat baca/simpan/edit ditampilkan di status bar, di samping info model yang masih terbuka, dan tidak membuat aplikasi crash.

## Struktur
- `camera.rs`: `Orbit`, logika kamera murni `f64`, dengan test unit tanpa GPU (jarak mata, batas pitch, pan mengikuti kursor, sinar lewat pikselnya, batas zoom, fit).
- `main.rs`: fungsi murni bertest unit (`union_bounds`, `wangun_path`, `save_target`, `write_atomic`, `typed_accepts`) dan app eframe 0.37 (wgpu). Viewport 3D dirender ke texture offscreen milik kita, lalu ditampilkan sebagai gambar egui. Dengan begitu depth buffer dikelola `render`, bukan egui.

- `smoke.rs` (hanya test): checklist `docs/user/uji-asap.md` dijalankan pada `App` asli dengan event egui mentah dan GPU offscreen, tanpa jendela. Mencakup keyboard, klik, seret, scroll, undo, simpan, buka ulang. Tombol toolbar tidak diklik lewat tata letak; kodenya hanya memanggil metode yang sama. Kotak path diklik sungguhan untuk menguji fokus keyboard. `WANGUNGIS_SMOKE_DUMP=<folder>` menulis gambar viewport untuk dilihat.

## Non-goals (menyusul)
- Dialog file native.
- Form pengisian origin.
- Lazy-load per tile.
