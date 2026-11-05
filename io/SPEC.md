# SPEC — `io` (wangungis-io)

## Tujuan
Menulis proyek ke format mesh yang dibaca aplikasi lain.

## API publik
- `export(&Project, format) -> Result<Vec<u8>, ExportError>`. `format`, tanpa membedakan huruf besar: `obj`, `stl`, `glb`, `gltf`, `3mf` (`FORMATS`).
- `ExportError { UnknownFormat, MultiTile, NothingToExport, Mesh, Hew }`.

## Perilaku
- **Koordinat:** meter lokal proyek, Z ke atas. Origin proyek tidak ditambahkan, supaya perangkat yang memakai `f32` tetap presisi. OBJ mencatat EPSG dan origin di komentar.
- **OBJ** ditulis di sini, karena Hew tidak punya. Satu objek per solid, segitiga, melewati semua tile.
- **STL, GLB, 3MF** memakai `mesh-export` Hew, yang bekerja per `Document`. Proyek dengan lebih dari satu tile ditolak (`MultiTile`).
- Hanya yang terlihat yang diekspor. Di OBJ, sketch tidak ikut. Di format Hew, perlakuannya mengikuti Hew (`solids_only = false`).

## Non-goals (menyusul)
- Impor. glTF/GLB impor dan CityJSON, CityGML, 3D Tiles (v0.2 dan seterusnya).
- Ekspor format Hew lintas tile.
