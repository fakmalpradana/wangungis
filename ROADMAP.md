# Roadmap

WangunGIS is built in milestones. A milestone is done when its exit criteria are met, not when a date arrives. The detailed planning documents are in Indonesian: [docs/plan/roadmap.md](docs/plan/roadmap.md) and the 100-feature [docs/plan/feature-matrix.md](docs/plan/feature-matrix.md). Feature numbers below (`#n`) refer to rows in that matrix, not to GitHub issues.

## Where we are

- **v0.1 (core modeling): in beta.** `v0.1.0-rc.2` is tagged for testing by a 3D engineer. Findings from that test decide what lands before `v0.1.0`.
- The geometry kernel is not ours. We use the crates `kernel`, `inference`, `tessellate` and `mesh-export` from Hew, pinned to one commit ([ADR-000](docs/adr/ADR-000-strategi-hew.md), [ADR-004](docs/adr/ADR-004-kernel-geometri.md)). Our crates wrap Hew and add what it lacks. Changing the pin needs the owner's approval.

## Milestones

### v0.1 — Core modeling (beta)
Drawing and editing tools, inference and snapping, groups, components, tags, outliner, paint, units, LAS point clouds with a slice box, autosave, 16 UI languages, binaries for 3 OS. See [CHANGELOG.md](CHANGELOG.md).

Exit criteria still open:
- A beta tester finishes the reference task (a 5×4×3 m box with a door, copied 10× in an array) on all 3 OS without errors.
- Fixes for what the beta test finds.

Deferred past v0.1: customizable shortcuts (#97), full parallel projection (#33).

### v0.2 — Geospatial + LOD1
CRS-aware projects (EPSG + `f64` origin, PROJ), reprojection on import, GeoJSON / Shapefile / DXF / GeoTIFF import, batch LOD1 extrusion from footprint heights, footprint cleanup, building attributes, CityJSON 2.0 export and import, headless CLI, culling and instancing.

Exit criteria:
- Import 10,000 footprints with heights into LOD1 in ≤ 30 s on an Apple M4.
- Projects in EPSG:32748/32749 (UTM, Indonesia) and EPSG:28992 (Netherlands) work without drift.
- CityJSON export passes `cjval` and `val3dity` 100%.
- Orbit at ≥ 60 FPS with 1M faces.

### v0.3 — LOD2
Roof templates (flat, shed, gable, hip; L/T/U footprints via straight skeleton; mansard; overhang), surface semantics (Ground/Wall/Roof) with automatic classification, live validation (watertight, planarity, orientation), boolean solids and Intersect Faces, CityGML 2.0 and 3D Tiles 1.1 export.

Exit criteria: LOD2 export passes `val3dity` 100%, and live validation reports the same error codes as the CLI validator. This is the first release promoted to the geospatial community.

### v0.4 — LOD3
Openings (windows, doors) as components, facade templates per floor, balconies and columns, multi-LOD per building with automatic generalization, textures, roof fitting from DSM or point cloud.

Exit criteria: LOD3 passes `val3dity` on ≥ 98% of a 50-building test set.

### v0.5+ — Remaining modeling parity, Python API, AI assist
Face styles (wireframe, X-ray), section planes, geolocated shadows, scenes, dimensions and text, mirror, protractor, PDF/SVG export, IFC export, a Python API as a separate process ([ADR-007](docs/adr/ADR-007-plugin.md)), then local ONNX roof and facade segmentation.

## Where a Rust / graphics developer can help

Open tasks are GitHub issues labelled [`help wanted`](https://github.com/fakmalpradana/wangungis/labels/help%20wanted); the smaller ones are also labelled [`good first issue`](https://github.com/fakmalpradana/wangungis/labels/good%20first%20issue). By area:

| Area | Crate | Examples |
|---|---|---|
| Rendering | `render` | Solid Push/Pull preview, wireframe and X-ray face styles (#35), culling and instancing (#37), section planes (#39) |
| Tools and inference | `tools` | Drawing on planes across group boundaries, mirror (#18), customizable shortcuts (#97) |
| Components | `tools`, `render` | Show other copies while one component is being edited |
| Point clouds | `pointcloud` | Per-class filtering, cache cleanup, LAZ (needs an ADR) |
| Packaging | `ui`, CI | macOS `.app` bundle with `icon.icns`, Linux `.desktop` file ([ADR-011](docs/adr/ADR-011-ikon-aplikasi.md)) |
| Performance | all | Benchmark harness and a regression threshold (the project has none yet) |
| Geospatial I/O (v0.2) | new `geo` crate | CityJSON 2.0 export (#84), CRS projects with PROJ (#57) |

Geometry-heavy work (Hew wrappers, inference, tolerances) is reviewed with extra care: see [CONTRIBUTING.md](CONTRIBUTING.md).
