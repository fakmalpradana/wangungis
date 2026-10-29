# ADR-003: Rendering dan presisi koordinat

- Status: Proposed (bergantung ADR-000)

## Keputusan
- Jalur B: **wgpu** (Metal/Vulkan/DX12; MIT/Apache-2.0).
- Jalur A: three.js milik Hew; migrasi ke WebGPU bila spike #2 gagal.
- Kedua jalur: model disimpan `f64`; render `f32` **relatif terhadap pusat lokal (RTC)** per bangunan/tile, dengan pusat dalam radius jauh di bawah ~131 km ([CESIUM_RTC](https://github.com/liwcv/glTF/blob/master/extensions/1.0/Vendor/CESIUM_RTC/README.md), [AGI](https://help.agi.com/AGIComponents/html/BlogPrecisionsPrecisions.htm)).

## Alternatif ditolak
- **bgfx**: C++, menambah FFI tanpa manfaat di atas wgpu.
- **Metal/Vulkan langsung**: tiga backend untuk satu orang.

## Konsekuensi
- Golden-image test wajib membandingkan render pada origin vs pada koordinat UTM.

## Bila opsi A2 dipilih (ADR-000, menunggu K2)
Keputusan untuk A2: wgpu dengan **batching** (buffer gabungan per tile) + RTC per tile ±5 km. Bukti spike (`research/spike-a2-native-render.md`): kota 1M face / 25 ribu Object = 203 FPS, sedangkan three.js Hew 8,6 FPS. Picking dan snapping memakai crate `inference` Hew yang di-shard per tile 100 m.
