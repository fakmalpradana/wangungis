# Draf issue untuk hew3d/hew (Hari 5, kriteria #6)

Status: **DRAF, belum diposting.** Posting butuh persetujuan manusia, karena issue ini publik dan atas nama akun `fakmalpradana`.

Kanal: GitHub Issues. Discussions tidak aktif di repo Hew (`has_discussions: false`). CONTRIBUTING Hew meminta issue dibuka lebih dulu untuk perubahan besar. Bahasa: Inggris.

Sebelum posting, isi tautan `<link>` di bawah. Hasil spike baru bisa ditautkan setelah repo WangunGIS dibuka publik. Bila repo masih private, ganti tautannya dengan potongan test langsung di dalam issue.

---

**Title:** Geospatial use case: large-coordinate precision and extension points for CRS/LOD (question before any PR)

**Body:**

Hi, and thanks for Hew. The kernel discipline (typed refusals, StateProof undo, determinism) is exactly what I'd want to build on.

**Context.** I'm a geospatial engineer working on an open-source tool for modelling buildings at LOD1–LOD3 (CityJSON/CityGML, validated with val3dity). I'm evaluating whether to build on Hew rather than start from scratch, since Hew already gets the SketchUp-style interaction right. Before proposing anything, I'd like to understand whether this scope fits Hew or should live entirely downstream.

**1. A precision finding at UTM coordinates.**
I ran a small local spike (it is not a PR) against v1.3.1 (`612c1c1`):
- Setup: a 5×4×3 m box, followed by 100 sequential push/pulls of 0.5–17 mm on its top and side faces.
- Axis-aligned boxes are fine everywhere.
- A box rotated 30° in plan, placed at northing ≈ 9,150,000 m (typical for UTM in the southern hemisphere), gets **3–13 of 100 push/pulls refused** with `PushPullError::NonManifoldResult`.
- In debug builds, the same case trips `check_invariants` ("face … has vertices off its stored plane") on the working clone.
- The same box near the origin, or at 10 km, has zero refusals, with errors around 1e-14 m.

My reading: `tol::PLANE_DIST` / `POINT_MERGE` = 1e-9 m, but one f64 ulp at 9.15e6 is ≈ 1.86e-9 m. A refitted plane cannot hold its own vertices within that tolerance. Release builds stay safe, because the refusal leaves the object untouched. Test source: `<link>`.

Downstream, I plan to keep geometry relative to a project origin, so this doesn't block me. Still, would you prefer one of these?
- (a) A documented assumption that kernel geometry lives near the origin.
- (b) A magnitude-relative tolerance.
- (c) Leave it as is.

I'm happy to turn the spike into a failing spec if that's useful.

**2. Extension points.**
The geospatial parts would need:
- A project CRS (EPSG code) plus an f64 origin stored in the document.
- Per-object semantic attributes (e.g. RoofSurface, WallSurface) that survive edits.
- An exporter from kernel Objects to CityJSON.

Questions:
- Would a CRS/origin field and semantic face attributes be acceptable in the `.hew` format? Or would you rather keep them in a separate sidecar or downstream crate?
- Is an out-of-tree crate that depends on `kernel` (the way the importers do) the shape you'd expect? Or is something like this better kept as a fork?

No pressure on timelines. I'd rather know early if this is out of scope.

---

*Catatan: jangan posting sampai manusia menyetujui. Setelah diposting, catat URL dan tanggalnya di ADR-000 (#6). Batas tunggu balasan 2 minggu.*
