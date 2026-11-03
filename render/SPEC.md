# SPEC — `render` (wangungis-render)

## Tujuan
Renderer wgpu ber-batching dengan RTC per tile (ADR-003). Satu tile kernel (satu `Document` Hew, ADR-004) menjadi satu tile render dengan dua draw call: face dan edge.

## API publik
- `TileMesh::from_document(&Document)`: menggabungkan geometri yang terlihat menjadi satu buffer face dan satu buffer edge, dengan posisi `f32` di frame tile. Isinya Object world, instance komponen lewat pose-nya, dan garis sketch. Node user-hidden dilewati. Region sketch tertutup belum punya isian (ponytail).
- `TileMesh::triangle_count()` dan `TileMesh::bounds()`.
- `Camera { eye, target, fovy, aspect, near, far }`, dengan `eye`/`target` dalam `f64` lokal-proyek.
- `Camera::view_proj_at_eye()`: rotasi view + proyeksi dengan mata di origin.
- `Renderer::new(&Device, TextureFormat)`: depth `DEPTH_FORMAT` (Depth32Float).
- `Renderer::add_tile(&Device, origin_f64, &TileMesh)`: mengunggah tile, menggantikan tile dengan origin yang sama. `Renderer::clear_tiles()`.
- `Renderer::set_overlay(&Device, &[OverlayLines { color, lines }])`: garis umpan balik tool. Satu warna per grup, satu draw call per grup, selalu di atas model, posisi `f64` lokal-proyek.
- `Renderer::prepare(&Queue, &Camera)` lalu `Renderer::draw(&mut RenderPass) -> draw_calls`.
- Renderer tidak tahu apa pun soal egui maupun UI.

## Invariant
- Offset tile terhadap kamera dihitung dalam `f64`. Hanya hasil kecilnya yang diubah ke `f32`.
- Render tile di origin, di 40 km, dan di UTM penuh (430000, 9150000) identik selama kamera ikut bergeser. Terukur 0,00% piksel berbeda; box di tepi tile (±1,6 km dari frame) 0,003%. Ambang: ≤ 0,1% (`tests/golden.rs`).
- Kontrol negatif: tanpa RTC, yaitu geometri `f32` di koordinat UTM, render salah (terukur 1,1–1,2% piksel berbeda, ambang > 0,5%; render tidak boleh kosong).
- Jumlah draw call = 2 × jumlah tile + jumlah grup overlay (buffer kosong dilewati). Uniform tiap tile terpisah: dua tile menghasilkan dua kotak di tempat yang benar.
- Matriks view tetap finite saat kamera tegak lurus ke bawah atau mata = target.
- Face diberi depth bias agar edge koplanar tidak z-fight.
- Test GPU di-skip bila tidak ada adapter atau device (hardware lalu fallback software), kecuali `WANGUNGIS_REQUIRE_GPU` di-set. CI men-set variabel itu di ketiga OS.

## Non-goals (menyusul)
- Patch buffer inkremental saat satu Object diedit: sekarang tile di-upload ulang.
- Isian region sketch, ketebalan garis overlay.
- Picking, MSAA, bayangan, material/tekstur.
