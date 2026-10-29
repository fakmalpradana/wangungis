Access date: 2026-10-28

# Raw Research: SketchUp User Pain Points, 3D City Modeling (LOD1–LOD3) Workflows, Hew, and Differentiation Opportunities

---

## SECTION A — SketchUp User Pain Points

### A1. Single-Threaded Performance Bottleneck on Large Models (Millions of Faces)
- **One-line description:** SketchUp's core geometry and viewport engine is single-threaded, causing severe viewport lag and stalls when orbiting or editing models with millions of faces or dense 3D Warehouse entourage.
- **Frequency of mention:** Very common (hundreds of threads across official forums and Reddit; 20+ threads sampled directly in search).
- **Details & community workarounds:** High core-count CPUs do not accelerate geometry calculation because edge/face updates run on a single thread ([forums.sketchup.com](https://forums.sketchup.com/t/how-do-you-keep-large-sketchup-models-running-smoothly/350331), [forums.sketchup.com](https://forums.sketchup.com/t/has-anyone-experienced-significant-improvements-in-sketchup-performance-after-upgrading-their-workstation-components/243580)). Users must disable edge profiles, shadows, and tags/layers, or purge unused components to keep models responsive ([forums.sketchup.com](https://forums.sketchup.com/t/sketchup-is-suddenly-very-slow/90698/11)).
- **Sources:**
  - https://forums.sketchup.com/t/how-do-you-keep-large-sketchup-models-running-smoothly/350331
  - https://forums.sketchup.com/t/has-anyone-experienced-significant-improvements-in-sketchup-performance-after-upgrading-their-workstation-components/243580

### A2. Precision & Small-Face Creation Failures ("The Dave Method" / Scale Up 1000x Workaround)
- **One-line description:** SketchUp merges vertices closer than approximately 0.001 inch (~0.0254 mm) and fails to create tiny edges or faces during intersections, extrusions, and tight-radius operations.
- **Frequency of mention:** Very common (one of the most frequently explained issues on the forum; 30+ threads seen, with dedicated tutorial threads).
- **Details & community workarounds:** When generating small bevels, threads, or roof/façade intersections, endpoints within the internal tolerance are treated as coincident, leaving missing faces ([community.sketchucation.com](https://community.sketchucation.com/topic/153712/why-the-generated-faces-are-too-small-for-sketchup-to-create)). Users rely on "The Dave Method": converting the object into a component, copying it, scaling the copy up by 100x–1000x, performing the edit on the giant instance, and deleting the giant copy ([forums.sketchup.com](https://forums.sketchup.com/t/the-dave-method/49720), [community.sketchucation.com](https://community.sketchucation.com/topic/155723/the-dave-method)).
- **Sources:**
  - https://forums.sketchup.com/t/the-dave-method/49720
  - https://community.sketchucation.com/topic/153712/why-the-generated-faces-are-too-small-for-sketchup-to-create

### A3. Discontinuation of SketchUp Make and Perpetual Classic Licenses (Subscription-Only Pricing)
- **One-line description:** Trimble discontinued the free desktop version (SketchUp Make 2017) and ended perpetual Classic Licenses on November 4, 2020, forcing users into recurring annual subscriptions or limited web-only tools.
- **Frequency of mention:** Very common (dozens of long megathreads on official forums and recurring top threads on r/Sketchup).
- **Details & community workarounds:** Independent professionals, woodworkers, and hobbyists express strong frustration over recurring subscription costs and loss of offline perpetual ownership ([help.sketchup.com](https://help.sketchup.com/en/end-of-classic-license-faq), [reddit.com](https://www.reddit.com/r/Sketchup/comments/hg8hsl/sketchup_pro_is_switching_to_100_subscription/)). Many hobbyists still hold onto legacy SketchUp Make 2017 installers to avoid annual fees ([forums.sketchup.com](https://forums.sketchup.com/t/sketchup-should-bring-back-classic-licences/348946/4), [reddit.com](https://www.reddit.com/r/Sketchup/comments/1q050zy/theres_no_way_anyone_is_actually_paying_this_right/)).
- **Sources:**
  - https://forums.sketchup.com/t/sketchup-should-bring-back-classic-licences/348946/4
  - https://www.reddit.com/r/Sketchup/comments/hg8hsl/sketchup_pro_is_switching_to_100_subscription/

### A4. Ruby API & Extension Breakage Across Major Version Upgrades
- **One-line description:** Major SketchUp releases that upgrade the embedded Ruby runtime break compiled C-extension binaries (`.so`/`.bundle`) and deprecated Ruby syntax, leaving critical third-party plugins unusable until authors update them.
- **Frequency of mention:** Common (spikes annually around major SketchUp releases; 15+ threads seen).
- **Details & community workarounds:** Extensions using compiled C/C++ libraries throw incompatible library errors when SketchUp upgrades its Ruby version ([forums.sketchup.com](https://forums.sketchup.com/t/important-upcoming-ruby-update/348793), [forums.sketchup.com](https://forums.sketchup.com/t/sketchup-ruby-incompatible-library-error/92776)). Unmaintained or hobbyist plugins become abandonware, sparking debates over compatibility responsibility ([forums.sketchup.com](https://forums.sketchup.com/t/extension-compatibility-who-is-responsible/94395)).
- **Sources:**
  - https://forums.sketchup.com/t/important-upcoming-ruby-update/348793
  - https://forums.sketchup.com/t/sketchup-ruby-incompatible-library-error/92776

### A5. Lack of Native GIS / CRS Georeferencing and "Far From Origin" Precision Breakdown
- **One-line description:** Beyond basic WGS84 "Add Location" imagery, SketchUp lacks native Coordinate Reference System (CRS/EPSG) management and suffers severe rendering jitter and clipping when geometry is placed at real-world UTM coordinates far from `(0,0,0)`.
- **Frequency of mention:** Common among urban planners, surveyors, and GIS/CAD users (15+ threads seen).
- **Details & community workarounds:** Importing CAD/GIS datasets with real-world state-plane or UTM coordinates places geometry hundreds of kilometers from the internal origin, causing OpenGL/GPU floating-point precision breakdown, erratic cursor snapping, and face distortion ([forums.sketchup.com](https://forums.sketchup.com/t/real-world-coordinates/340782), [forums.sketchup.com](https://forums.sketchup.com/t/coordinates-too-far-from-the-origin/345525)). Users must manually strip false eastings/northings in QGIS or FME before import and re-apply offsets on export ([forums.sketchup.com](https://forums.sketchup.com/t/real-world-coordinates/340782)).
- **Sources:**
  - https://forums.sketchup.com/t/real-world-coordinates/340782
  - https://forums.sketchup.com/t/coordinates-too-far-from-the-origin/345525

### A6. Poor Native IFC Export Quality and Lack of Native CityGML/CityJSON Support
- **One-line description:** Native IFC export requires tedious manual schema classification and struggles with custom property sets (`Psets`), while CityGML/CityJSON is not supported natively at all.
- **Frequency of mention:** Occasional to common in BIM/GIS workflows (10+ threads seen).
- **Details & community workarounds:** Users must manually wrap geometry into components and assign IFC classes via the Classifier tool, and 2D lines or unthickened faces fail to export properly ([forums.sketchup.com](https://forums.sketchup.com/t/export-to-ifc-how-to-export-lines-and-edit-component-info/50351)). Reliable attribute mapping requires third-party plugins like `SketchUp IFC Manager` ([forums.sketchup.com](https://forums.sketchup.com/t/proper-ifc-export-with-mapping-from-sketchup-to-ifc-attributes/64155)), and CityGML workflows require external commercial plugins like `CityEditor` or FME pipelines ([forums.sketchup.com](https://forums.sketchup.com/t/citygml-plugins-for-sketchup/24921)).
- **Sources:**
  - https://forums.sketchup.com/t/proper-ifc-export-with-mapping-from-sketchup-to-ifc-attributes/64155
  - https://forums.sketchup.com/t/citygml-plugins-for-sketchup/24921

### A7. Non-Manifold / Non-Solid Group Errors Blocking Boolean & 3D Printing Workflows
- **One-line description:** Groups and components frequently lose "Solid" (2-manifold) status due to stray edges, internal partition faces, micro-holes, or nested sub-groups, causing boolean tools and downstream validators to reject them.
- **Frequency of mention:** Very common (dozens of threads across SketchUp and SketchUcation forums; 25+ threads seen).
- **Details & community workarounds:** For SketchUp to recognize a manifold solid, every edge must border exactly two faces with no stray lines or nested groups ([community.sketchucation.com](https://community.sketchucation.com/topic/164097/discussion-object-is-not-a-manifold-solid), [forums.sketchup.com](https://forums.sketchup.com/t/group-not-a-solid/344932)). Users depend heavily on ThomThom's `Solid Inspector2` and `CleanUp3` extensions, though micro-gaps can still cause SketchUp's native solid check to disagree with `Solid Inspector2` ([forums.sketchup.com](https://forums.sketchup.com/t/solid-inspector-shows-good-but-it-is-not-solid/206705)).
- **Sources:**
  - https://forums.sketchup.com/t/solid-inspector-shows-good-but-it-is-not-solid/206705
  - https://community.sketchucation.com/topic/164097/discussion-object-is-not-a-manifold-solid

### A8. Native Solid Tools Destroying Component Definitions (Converting Components to Groups)
- **One-line description:** Executing native Solid Tools (Union, Subtract, Trim, Split) on a Component instance strips its component definition and replaces it with a generic Group, breaking instance linking across the model.
- **Frequency of mention:** Common among intermediate/advanced modelers (10+ threads seen).
- **Details & community workarounds:** Users who organize repeating architectural elements as Components find that native boolean operations sever the link to other instances and rename the result "Difference" or "Union" as a Group ([forums.sketchup.com](https://forums.sketchup.com/t/why-does-solid-tool-turn-component-into-group/228560)). Workarounds require installing third-party plugins such as `Eneroth Solid Tools` or paid `BoolTools 2` ([forums.sketchup.com](https://forums.sketchup.com/t/why-does-solid-tool-turn-component-into-group/228560)).
- **Sources:**
  - https://forums.sketchup.com/t/why-does-solid-tool-turn-component-into-group/228560
  - https://extensions.sketchup.com/extension/06e6d25a-7c3c-45f6-9aae-b777982956e1/eneroth-component-to-group

### A9. "Intersect Faces" Failing to Split Faces or Leaving Missing Edge Loops
- **One-line description:** The `Intersect Faces` command frequently leaves unclosed edge loops, fails to divide coplanar or near-tangent surfaces, or drops tiny segments below SketchUp's tolerance threshold.
- **Frequency of mention:** Very common (20+ threads seen across official forums and SketchUcation).
- **Details & community workarounds:** Intersecting geometry fails to cut faces if executed in the wrong edit context, if resulting segments fall below 0.001 inch, or when coplanar edges overlap existing face boundaries ([forums.sketchup.com](https://forums.sketchup.com/t/trouble-with-intersect-faces/35124), [forums.sketchup.com](https://forums.sketchup.com/t/intersect-faces-not-creating-faces-on-all-edges/231744/4)). Users must manually re-trace edges with the Line tool or scale models up 1000x before intersecting ([community.sketchucation.com](https://community.sketchucation.com/topic/154791/intersect-faces-not-working)).
- **Sources:**
  - https://forums.sketchup.com/t/intersect-faces-not-creating-faces-on-all-edges/231744/4
  - https://community.sketchucation.com/topic/154791/intersect-faces-not-working

### A10. "Follow Me" Tool Failures on Tight Corners and 3D Paths
- **One-line description:** Extruding profiles with `Follow Me` around tight inner radii or 3D non-planar paths causes missing faces, self-intersecting "butterfly" folds, or unexpected profile twisting.
- **Frequency of mention:** Very common (25+ threads seen).
- **Details & community workarounds:** When a profile sweeps around an arc whose radius is smaller than the profile width, edges self-intersect or shrink below the tiny-face threshold, leaving gaps ([forums.sketchup.com](https://forums.sketchup.com/t/issue-with-follow-me-tool-and-missing-surfaces/315922), [forums.sketchup.com](https://forums.sketchup.com/t/problems-with-the-follow-me-tool/177767)).Sweeping along 3D helical or sloped paths also twists the cross-section, forcing users to rely on plugins like `Eneroth Upright Extruder` or `FredoCorner` ([community.sketchucation.com](https://community.sketchucation.com/topic/148252/trouble-with-follow-me-tool-around-curved-edge-with-file)).
- **Sources:**
  - https://forums.sketchup.com/t/issue-with-follow-me-tool-and-missing-surfaces/315922
  - https://community.sketchucation.com/topic/148252/trouble-with-follow-me-tool-around-curved-edge-with-file

### A11. "Sticky Geometry" (Unintended Merging of Ungrouped Edges and Faces)
- **One-line description:** All raw edges and faces in the same editing context automatically weld and split each other on contact, causing severe accidental mesh distortion if users forget to group geometry immediately.
- **Frequency of mention:** Very common (universal beginner trap and frequent critique of SketchUp's architecture; 20+ threads seen).
- **Details & community workarounds:** Unlike solids-based CAD tools where new extrusions are discrete bodies, touching raw geometry in SketchUp permanently fuses together; moving one face stretches and tears adjacent geometry ([forums.sketchup.com](https://forums.sketchup.com/t/beginner-problems-merging-objects-copiyng/134923)). Forum veterans enforce the rule to wrap every element into a Group or Component before drawing the next ([forums.sketchup.com](https://forums.sketchup.com/t/beginner-problems-merging-objects-copiyng/134923/11)).
- **Sources:**
  - https://forums.sketchup.com/t/beginner-problems-merging-objects-copiyng/134923
  - https://forums.sketchup.com/t/beginner-problems-merging-objects-copiyng/134923/11

### A12. Lack of Native Linux Desktop Support
- **One-line description:** Trimble provides no native Linux build of SketchUp Desktop, leaving Linux users dependent on the feature-limited Web version, Virtual Machines, or fragile Wine/Bottles setups.
- **Frequency of mention:** Common among Linux/open-source users (15+ threads seen).
- **Details & community workarounds:** Running SketchUp Pro or Make 2017 under Linux via Wine/Bottles suffers from `.NET` runtime installation hurdles, GPU driver glitches, and broken web dialogs ([forums.sketchup.com](https://forums.sketchup.com/t/how-to-use-sketchup-on-linux/131201), [forums.sketchup.com](https://forums.sketchup.com/t/sketchup-2025-installation-wine-bottle-and-ubuntu-25-10-linux/344029)). Browser-based SketchUp Free works on Linux but lacks Ruby extensions and offline native performance ([reddit.com](https://www.reddit.com/r/Sketchup/comments/ssj46r/sketchup_on_linux/)).
- **Sources:**
  - https://forums.sketchup.com/t/how-to-use-sketchup-on-linux/131201
  - https://forums.sketchup.com/t/sketchup-2025-installation-wine-bottle-and-ubuntu-25-10-linux/344029

### A13. LayOut Sluggishness, Beachballing, and Vector Viewport Lag
- **One-line description:** SketchUp's 2D documentation companion, LayOut, becomes severely laggy when zooming, panning, or switching pages with complex models, especially in Vector or Hybrid rendering modes.
- **Frequency of mention:** Very common (one of the #1 complaints from professional architects on both `forums.sketchup.com` and `r/Sketchup`; 30+ threads seen).
- **Details & community workarounds:** Vector and Hybrid viewports recompute hidden-line removal across every edge in the model, triggering multi-second freezes or macOS spinning beachballs ([forums.sketchup.com](https://forums.sketchup.com/t/why-does-the-spinning-beach-ball-appear-so-much-and-it-take-so-long-to-switch-between-layout-sheets/79166/8), [reddit.com](https://www.reddit.com/r/Sketchup/comments/hbcj1s/layout_is_so_damn_slow/)). Users work in low-resolution Raster mode with auto-render disabled and stack viewports as a workaround ([forums.sketchup.com](https://forums.sketchup.com/t/layout-slowing-down/343406), [reddit.com](https://www.reddit.com/r/Sketchup/comments/1n9vdqc/problem_with_sketchup_layout_performance/)).
- **Sources:**
  - https://forums.sketchup.com/t/why-does-the-spinning-beach-ball-appear-so-much-and-it-take-so-long-to-switch-between-layout-sheets/79166/8
  - https://www.reddit.com/r/Sketchup/comments/hbcj1s/layout_is_so_damn_slow/

### A14. Undo Slowness, UI Freezes, and Undo-Stack Crashes in Heavy/Extension Operations
- **One-line description:** Heavy geometry operations or Ruby scripts that do not suppress UI redraws freeze the application, and undoing large operations or attribute dictionary changes can stall or crash SketchUp.
- **Frequency of mention:** Occasional to common (10+ threads seen, especially in Ruby API and heavy-modeling discussions).
- **Details & community workarounds:** If a Ruby extension calls `model.start_operation` without setting `disable_ui = true`, SketchUp redraws the UI after each entity modification, freezing the app ([forums.sketchup.com](https://forums.sketchup.com/t/how-not-to-freeze-sketchup-when-ruby-is-running/171079/2)). Furthermore, executing thousands of sequential operations or undoing attribute dictionary modifications has historically triggered BugSplat crashes ([forums.sketchup.com](https://forums.sketchup.com/t/sketchup-crash-during-a-lot-of-operations/254240), [forums.sketchup.com](https://forums.sketchup.com/t/mysterious-crash-when-undoing-attribute-changes/52575)).
- **Sources:**
  - https://forums.sketchup.com/t/how-not-to-freeze-sketchup-when-ruby-is-running/171079/2
  - https://forums.sketchup.com/t/sketchup-crash-during-a-lot-of-operations/254240

### A15. Camera Clipping Plane Disappearing Geometry Bug
- **One-line description:** Zooming in on small details within a large site model or switching between Perspective and Parallel Projection causes the near camera clipping plane to slice away visible geometry.
- **Frequency of mention:** Very common (25+ threads seen).
- **Details & community workarounds:** When a model has large spatial extents, stray distant edges, or a skewed Parallel Projection camera distance, zooming in slices through the front of the model ([forums.sketchup.com](https://forums.sketchup.com/t/disappearing-geometry/127232), [forums.sketchup.com](https://forums.sketchup.com/t/clipping-bug-when-in-parallel-projection/119465)). Users must repeatedly toggle back to Perspective mode and press `Zoom Extents` or hunt down stray far-away geometry ([community.sketchucation.com](https://community.sketchucation.com/topic/136526/i-ve-got-a-problem-with-sketchup-clipping-plane-issues)).
- **Sources:**
  - https://forums.sketchup.com/t/clipping-bug-when-in-parallel-projection/119465
  - https://community.sketchucation.com/topic/136526/i-ve-got-a-problem-with-sketchup-clipping-plane-issues

### A16. Z-Fighting on Coplanar Faces and Inconsistent Face Normal Orientation (Reversed Faces)
- **One-line description:** Coplanar overlapping faces cause severe depth-buffer flickering (Z-fighting), while extrusions and intersections frequently leave reversed (back-side outward) face normals that break renderers and GIS/solid exports.
- **Frequency of mention:** Very common (20+ threads seen).
- **Details & community workarounds:** SketchUp does not automatically resolve coplanar faces across separate groups, resulting in flickering surfaces ([forums.sketchup.com](https://forums.sketchup.com/t/why-does-sketchup-do-this/214828), [community.sketchucation.com](https://community.sketchucation.com/topic/152666/z-fighting-on-non-coplanar-faces)). Additionally, Push/Pull and Follow Me often leave blue back-faces pointing outward, requiring users to constantly inspect in Monochrome mode and manually run `Reverse Faces` / `Orient Faces`.
- **Sources:**
  - https://forums.sketchup.com/t/why-does-sketchup-do-this/214828
  - https://community.sketchucation.com/topic/152666/z-fighting-on-non-coplanar-faces

### A17. Faceted/Segmented Arcs and Circles Instead of True Analytical Curves
- **One-line description:** SketchUp represents all arcs and circles as faceted polyline segments rather than analytical curves or NURBS, causing faceted 3D geometry and lost curve metadata in 2D/3D CAD exports.
- **Frequency of mention:** Common among CAD/CNC and precision architectural users (15+ threads seen).
- **Details & community workarounds:** If an arc or circle is split, intersected, or exported via 2D DXF/DWG (or 3D DXF with faces enabled), SketchUp loses its curve metadata and exports broken straight line segments instead of true CAD arcs ([forums.sketchup.com](https://forums.sketchup.com/t/dxf-output-and-curves/296105), [forums.sketchup.com](https://forums.sketchup.com/t/sketchup-23-dxf-export-issue-with-curves/241757)).
- **Sources:**
  - https://forums.sketchup.com/t/dxf-output-and-curves/296105
  - https://forums.sketchup.com/t/sketchup-23-dxf-export-issue-with-curves/241757

---

## SECTION B — LOD1/LOD2/LOD3 Building Modeling Workflows in Practice (CityGML / CityJSON)

### B1. `3dfier` (TU Delft 3D Geoinformation Group) — Automated LOD1 Extrusion & Terrain Stitching
- **How practitioners use it:** `3dfier` takes 2D GIS topographic/cadastral polygon datasets (such as building footprints, water, and roads) and airborne LiDAR point clouds (LAS/LAZ) and lifts each polygon to 3D, extruding buildings to flat-roofed LOD1 blocks based on point-cloud height percentiles while stitching a watertight Digital Surface Model in CityJSON, CityGML, OBJ, or CSV ([tudelft3d.github.io](https://tudelft3d.github.io/3dfier/generate_lod1.html), [github.com](https://github.com/tudelft3d/3dfier)).
- **Pain points:**
  - Strictly limited to LOD1 prism extrusion; cannot reconstruct sloped/complex roofs (LOD2) or façade openings (LOD3) ([tudelft3d.github.io](https://tudelft3d.github.io/3dfier/generate_lod1.html)).
  - 2D topological errors, self-intersections, or sliver polygons in the source cadastral footprints directly propagate into 3D validation failures in `val3dity` ([tudelft3d.github.io](https://tudelft3d.github.io/3dfier/generate_lod1.html)).
  - CLI-only batch pipeline with no interactive 3D editing UI to fix individual problematic buildings ([github.com](https://github.com/tudelft3d/3dfier)).
- **Sources:**
  - https://tudelft3d.github.io/3dfier/generate_lod1.html
  - https://github.com/tudelft3d/3dfier

### B2. `geoflow` / `roofer` & the 3D BAG Pipeline (TU Delft / 3DGI) — Automated National-Scale LOD1.2, LOD1.3, and LOD2.2
- **How practitioners use it:** Used to produce the Dutch **3D BAG** (`3dbag.nl`, ~10 million buildings) by combining 2D BAG building footprints with AHN airborne point clouds ([docs.3dbag.nl](https://docs.3dbag.nl/en/), [research.tudelft.nl](https://research.tudelft.nl/en/datasets/3d-bag)). Originally built on the flowchart-based C++ `geoflow3d` engine (`gfp-building-reconstruction`) and now refactored into `roofer`, it automatically detects roof planes via RANSAC/plane-fitting and reconstructs LoD1.2, LoD1.3 (stepped flat roofs), and LoD2.2 (multi-pitched roofs with semantics) in CityJSON and GeoPackage ([innovation.3dbag.nl](https://innovation.3dbag.nl/roofer/), [github.com](https://github.com/3DBAG/roofer)).
- **Pain points:**
  - Relies on high-density airborne LiDAR; sparse or occluded point clouds cause missing roof planes, degenerate faces, or fallback to simple LoD1 blocks ([docs.3dbag.nl](https://docs.3dbag.nl/en/)).
  - Cannot reconstruct under-roof overhangs, façade recesses, windows, or doors (stops at LoD2.2 because airborne nadir LiDAR rarely captures vertical façades) ([docs.3dbag.nl](https://docs.3dbag.nl/en/)).
  - Fully automated batch tool; when a complex landmark building reconstructs poorly, practitioners lack an interactive CAD/SketchUp-like tool to manually refine the LoD2.2 solid while keeping CityJSON semantics and `val3dity` validity intact ([cityjson.org](https://www.cityjson.org/news/2021/04/07/release-3dbag/)).
- **Sources:**
  - https://docs.3dbag.nl/en/
  - https://innovation.3dbag.nl/roofer/
  - https://github.com/3DBAG/roofer

### B3. `val3dity` & `cjio` (TU Delft / CityJSON Ecosystem) — ISO 19107 3D Geometric & Topological Validation
- **How practitioners use it:** Practitioners and national mapping agencies run `val3dity` (standalone CLI or via `cjio` / web app) as the gold-standard validator to verify that 3D primitives (`MultiSurface`, `CompositeSurface`, `Solid`) in CityJSON, CityGML, and OBJ conform to ISO 19107 rules ([github.com](https://github.com/tudelft3d/val3dity), [cityjson.org](https://www.cityjson.org/tutorials/validation/)).
- **Pain points:**
  - **Non-planar faces (`203 NON_PLANAR_POLYGON_DISTANCE_PLANE` and `204 NON_PLANAR_POLYGON_NORMALS_DEVIATION`):** Standard 3D modelers (SketchUp, Blender, Rhino) allow slightly warped quads/n-gons or floating-point rounding drift, which immediately fail `val3dity`'s least-squares plane distance check (default 1 cm tolerance) or triangle normal deviation check ([val3dity.readthedocs.io](https://val3dity.readthedocs.io/2.6.0/errors/)).
  - **Unsealed Solids & Orientation Errors (`302 SHELL_NOT_CLOSED`, `307 POLYGON_WRONG_ORIENTATION`):** Manual modeling tools rarely enforce outward-pointing normals and watertight 2-manifold shells across every edit, causing high rejection rates when validating manually edited CityJSON/CityGML models ([val3dity.readthedocs.io](https://val3dity.readthedocs.io/2.6.0/errors/)).
  - `val3dity` only *reports* error codes and offending vertex/face IDs; it does not interactively repair the geometry ([github.com](https://github.com/tudelft3d/val3dity)).
- **Sources:**
  - https://val3dity.readthedocs.io/2.6.0/errors/
  - https://github.com/tudelft3d/val3dity
  - https://www.cityjson.org/tutorials/validation/

### B4. Blender + `Up3date` / `BlenderGIS` — Interactive Mesh Editing of CityJSON & GIS Data
- **How practitioners use it:** Practitioners use `BlenderGIS` to import basemaps, DEM rasters, and 2D shapefiles, and use the TU Delft-developed `Up3date` add-on (`cityjson/Up3date`) to import, inspect, edit, and export CityJSON models (including 3D BAG files) inside Blender while mapping semantic surfaces (`RoofSurface`, `WallSurface`, `GroundSurface`) to Blender materials ([github.com](https://github.com/cityjson/Up3date), [community.osarch.org](https://community.osarch.org/discussion/381/import-cityjson-workflows)).
- **Pain points:**
  - Blender's mesh editing tools are designed for artistic polygon modeling, not precision CAD inference; moving a vertex easily warps a multi-vertex roof or wall polygon out of planarity, triggering `val3dity` error `203`/`204` on export ([val3dity.readthedocs.io](https://val3dity.readthedocs.io/2.6.0/errors/)).
  - Preserving large CRS coordinates requires local origin shifting (`transform` object in CityJSON), and editing parent-child hierarchies (`Building` -> `BuildingPart` -> `BuildingInstallation`) in Blender's Outliner is fragile and error-prone ([github.com](https://github.com/cityjson/Up3date)).
- **Sources:**
  - https://github.com/cityjson/Up3date
  - https://community.osarch.org/discussion/381/import-cityjson-workflows

### B5. Esri CityEngine (CGA Procedural Shape Grammar Rules) — Rule-Based LOD2/LOD3 Generation
- **How practitioners use it:** Urban designers write CGA (Computer Generated Architecture) shape grammar rules to procedurally extrude 2D footprints, generate parametric roof types (gable, hip, mansard), and split façades into repetitive LOD3 window/balcony tiles, exporting the results to CityGML or 3D Tiles ([isprs-annals.copernicus.org](https://isprs-annals.copernicus.org/articles/IV-2-W1/13/2016/isprs-annals-IV-2-W1-13-2016.pdf), [community.esri.com](https://community.esri.com/t5/arcgis-cityengine-questions/ce-city-does-not-generate-buildings-when-uploaded/td-p/620247)).
- **Pain points:**
  - **Proprietary cost & steep learning curve:** Requires an expensive commercial Esri license and text-based CGA programming rather than direct push/pull geometric manipulation ([community.esri.com](https://community.esri.com/t5/arcgis-cityengine-questions/ce-city-does-not-generate-buildings-when-uploaded/td-p/620247)).
  - **Invalid Solids & Missing Semantics in CityGML Export:** CGA rules optimize for visual appearance; overlapping volumetric extrusions, unsealed undersides, or untagged `comp(f)` splits produce non-manifold shells, self-intersecting faces, and missing `WallSurface`/`RoofSurface` hierarchies that fail 3DCityDB and `val3dity` checks ([github.com](https://github.com/3dcitydb/importer-exporter/issues/57), [isprs-annals.copernicus.org](https://isprs-annals.copernicus.org/articles/IV-2-W1/13/2016/isprs-annals-IV-2-W1-13-2016.pdf)).
- **Sources:**
  - https://isprs-annals.copernicus.org/articles/IV-2-W1/13/2016/isprs-annals-IV-2-W1-13-2016.pdf
  - https://github.com/3dcitydb/importer-exporter/issues/57

### B6. Safe Software FME — Enterprise ETL for CityGML/CityJSON Validation, Repair, and Translation
- **How practitioners use it:** Municipalities and GIS engineers use FME Form/Flow as the central pipeline to read/write CityGML and CityJSON, reproject CRS coordinates, run `GeometryValidator` to detect non-planar or unsealed solids, heal normals/gaps, and attach semantic `WallSurface`, `RoofSurface`, and `GroundSurface` relations ([docs.safe.com](https://docs.safe.com/fme/html/FME-Form-Documentation/FME-ReadersWriters/cityjson/cityjson.htm), [community.safe.com](https://community.safe.com/data-7/citygml-data-geometry-repair-13300)).
- **Pain points:**
  - High proprietary licensing cost and complex node-graph workbenches required even for simple CityGML hierarchy manipulations ([community.safe.com](https://community.safe.com/data-7/citygml-data-geometry-repair-13300)).
  - Automated geometry repair (`GeometryValidator` / `SurfaceModeller`) often triangulates planar faces into dense TIN meshes or fails to reconstruct valid closed `gml:Solid` volumes when source CAD/SketchUp models contain internal walls or self-intersections ([community.safe.com](https://community.safe.com/data-7/citygml-data-geometry-repair-13300)).
- **Sources:**
  - https://docs.safe.com/fme/html/FME-Form-Documentation/FME-ReadersWriters/cityjson/cityjson.htm
  - https://community.safe.com/data-7/citygml-data-geometry-repair-13300

### B7. SketchUp + `CityEditor` Plugin (3DIS GmbH) — Manual LOD1–LOD3 Modeling & Texturing in SketchUp
- **How practitioners use it:** Practitioners use the commercial `CityEditor` extension inside SketchUp Pro (Windows-only) to import CityGML files, model or refine building blocks and roofs using SketchUp's Push/Pull tools, classify faces into CityGML boundary surfaces (`RoofSurface`, `WallSurface`, `GroundSurface`, `ClosureSurface`), and export back to CityGML ([extensions.sketchup.com](https://extensions.sketchup.com/extension/e14da71b-ee5d-4a68-950d-3193d182d195/0), [assets.3dis.de](https://assets.3dis.de/CityEditor_en.pdf)).
- **Pain points:**
  - Inherits all of SketchUp's core geometric flaws: sticky geometry (A11), tiny-face dropouts (A2), reversed normals (A16), and non-solid groups (A7), frequently exporting CityGML files with non-planar polygons or open shells ([assets.3dis.de](https://assets.3dis.de/CityEditor_en.pdf)).
  - Manual semantic assignment face-by-face is tedious and error-prone on large building stocks, and `CityEditor` is commercial and Windows-only ([extensions.sketchup.com](https://extensions.sketchup.com/extension/e14da71b-ee5d-4a68-950d-3193d182d195/0), [assets.3dis.de](https://assets.3dis.de/CityEditor_en.pdf)).
- **Sources:**
  - https://extensions.sketchup.com/extension/e14da71b-ee5d-4a68-950d-3193d182d195/0
  - https://assets.3dis.de/CityEditor_en.pdf

### B8. KIT `FZKViewer` / `KITModelViewer` — Semantic & Visual Inspection of CityGML and IFC
- **How practitioners use it:** Researchers and GIS practitioners use the free Windows tool `FZKViewer` (and its successor `KITModelViewer`, developed at Karlsruhe Institute of Technology) to visually inspect CityGML (v0.4–v3.0) and IFC files, browse the semantic object tree (`Building` -> `BoundarySurfaces` -> `Opening`), and color-code surfaces by semantic class or attribute ([iai.kit.edu](https://www.iai.kit.edu/english/1648.php), [iai.kit.edu](https://www.iai.kit.edu/english/4561.php)).
- **Pain points:**
  - Strictly a read-only viewer/inspector with no geometric editing capabilities to fix the errors a user spots ([iai.kit.edu](https://www.iai.kit.edu/english/1648.php)).
  - Windows-only binary and not a strict ISO 19107 topological validator like `val3dity` ([iai.kit.edu](https://www.iai.kit.edu/english/1648.php)).
- **Sources:**
  - https://www.iai.kit.edu/english/1648.php
  - https://www.iai.kit.edu/english/4561.php

### B9. Rhino / Grasshopper & Academic LOD3 Workflows (TUM / TU Delft) — Parametric & Manual LOD3 Façade Modeling
- **How practitioners use it:** For LOD3 models (detailed façades, windows, doors, balconies, roof overhangs), practitioners and research groups (such as TUM's `TUM2TWIN` / `GT-LOD3` benchmarks and TU Delft's IFC-to-CityGML LOD3 pipelines) either convert architectural BIM/IFC models or manually/parametrically trace openings over Mobile Laser Scanning (MLS) point clouds and terrestrial imagery in CAD tools like Rhino/Grasshopper or SketchUp ([tum2t.win](https://tum2t.win/datasets/cm-buildings), [research.tudelft.nl](https://research.tudelft.nl/en/publications/automatic-generation-of-citygml-lod3-building-models-from-ifc-mod/), [github.com](https://github.com/gdslab/GT-LOD3-Benchmark)).
- **Pain points:**
  - **Extreme manual effort for LOD3:** Creating ground-truth LOD3 buildings requires hours of manual labor per building to cut windows, doors, and recesses into LOD2 shells while maintaining valid topological openings ([tum2t.win](https://tum2t.win/datasets/cm-buildings), [github.com](https://github.com/gdslab/GT-LOD3-Benchmark)).
  - **Fragile CAD-to-GIS semantic mapping:** Rhino/Grasshopper works with NURBS/B-Reps and lacks native CityGML/CityJSON semantic hierarchies (`WallSurface` hosting `Window`/`Door` openings), requiring custom Python/FME scripts that often fail `val3dity` solid checks due to coplanar window surfaces or floating-origin precision loss ([isprs-archives.copernicus.org](https://isprs-archives.copernicus.org/articles/L-4-W2-2026/95/2026/isprs-archives-L-4-W2-2026-95-2026.pdf), [research.tudelft.nl](https://research.tudelft.nl/en/publications/automatic-generation-of-citygml-lod3-building-models-from-ifc-mod/)).
- **Sources:**
  - https://tum2t.win/datasets/cm-buildings
  - https://research.tudelft.nl/en/publications/automatic-generation-of-citygml-lod3-building-models-from-ifc-mod/
  - https://github.com/gdslab/GT-LOD3-Benchmark

---

## SECTION C — Hew (`github.com/hew3d/hew`) User Discussions & Findings

**Search coverage summary:** Searched GitHub (`github.com/hew3d/hew`), Reddit (`site:reddit.com/r/Sketchup`), AlternativeTo (`alternativeto.net`), and Hacker News (`site:news.ycombinator.com` for `"hew3d"`, `"hew3d.com"`, and `"github.com/hew3d/hew"`). Active repository documentation and multiple Reddit community release/feedback threads were found on `r/Sketchup`; **no Hacker News discussions were found** on `news.ycombinator.com`.

### C1. Hew Architecture & Solids-First Design (`github.com/hew3d/hew`)
- **Findings:** Hew is an open-source, cross-platform (macOS, Windows, Linux, and WebGL2 browser at `app.hew3d.com`) 3D modeler designed around SketchUp's draw-and-push/pull UX, inference snapping, and VCB measurement inputs ([github.com](https://github.com/hew3d/hew), [alternativeto.net](https://alternativeto.net/software/hew/about/)). Unlike SketchUp's "sticky" surface mesh model, Hew uses a **solids-first architecture** where every extruded closed profile produces a discrete watertight solid that only combines via explicit Boolean operations (Union, Subtract, Intersect) ([github.com](https://github.com/hew3d/hew)).
- **Sources:**
  - https://github.com/hew3d/hew
  - https://alternativeto.net/software/hew/about/

### C2. Native `.skp` Interoperability (`openskp`) and Format Support
- **Findings:** In Reddit discussions on `r/Sketchup` (covering Hew 1.1, 1.2 "Going Native", and 1.3 "Second Nature"), the creator and users highlighted clean-room `.skp` file reading and writing (supporting both legacy SketchUp Make 2017 and newer 2026 `.skp` files), plus exports to STL, 3MF, glTF, USDZ, and SVG ([reddit.com](https://www.reddit.com/r/Sketchup/comments/1x52vjx/update_hew_12_the_going_native_release_opens/), [reddit.com](https://www.reddit.com/r/Sketchup/comments/1wsunyd/request_complex_sketchup_models_2026_or_2017_to/)).
- **Sources:**
  - https://www.reddit.com/r/Sketchup/comments/1x52vjx/update_hew_12_the_going_native_release_opens/
  - https://www.reddit.com/r/Sketchup/comments/1wsunyd/request_complex_sketchup_models_2026_or_2017_to/

### C3. Community Feedback on `r/Sketchup` (Muscle Memory, Woodworking/3D Printing Focus)
- **Findings:** User feedback in `r/Sketchup` threads praised the availability of a free, native Linux/macOS/Windows desktop alternative to SketchUp Make 2017 and requested muscle-memory refinements such as custom camera orbit/pan behavior, selection modifiers, cut-list generation, and print-to-scale 2D tiling ([reddit.com](https://www.reddit.com/r/Sketchup/comments/1wi7rky/update_hew_11_the_user_feedback_release_is_out/), [reddit.com](https://www.reddit.com/r/Sketchup/comments/1xegym8/update_hew_13_the_second_nature_release_is_out/)).
- **Sources:**
  - https://www.reddit.com/r/Sketchup/comments/1wi7rky/update_hew_11_the_user_feedback_release_is_out/
  - https://www.reddit.com/r/Sketchup/comments/1xegym8/update_hew_13_the_second_nature_release_is_out/

### C4. Gap in Hew for Geospatial / 3D City Modeling (LOD1–LOD3, CityJSON/CityGML)
- **Findings:** Based on `github.com/hew3d/hew` and `r/Sketchup` threads, Hew targets general fabrication, woodworking (cut lists), and 3D printing (STL/3MF) and has no GIS coordinate reference system (CRS) support, no floating-origin handling for UTM coordinates, no semantic surface hierarchy (`RoofSurface`/`WallSurface`/`Opening`), and no CityJSON, CityGML, IFC, or `val3dity` integration ([github.com](https://github.com/hew3d/hew), [reddit.com](https://www.reddit.com/r/Sketchup/comments/1x3gbpq/i_am_working_on_a_free_sketchup_inspired_cad_with/)).
- **Sources:**
  - https://github.com/hew3d/hew
  - https://www.reddit.com/r/Sketchup/comments/1x3gbpq/i_am_working_on_a_free_sketchup_inspired_cad_with/

---

## SECTION D — Differentiation Opportunities for a New Open-Source LOD1–LOD3 Modeler

### D1. Double-Precision Kernel with Automatic Floating Origin & Native CRS/PROJ Support
- **References:** Fixes `A2`, `A5`, `A9`, `A15`, `B4`, `B7`, `B9`, and addresses the geospatial gap in `C4`.
- **Opportunity:** By combining double-precision world coordinates with automatic origin shifting (floating origin) and native `PROJ` CRS transformations, users can import and edit buildings directly in national UTM/EPSG grids without suffering SketchUp's "far from origin" viewport jitter (`A5`, [forums.sketchup.com](https://forums.sketchup.com/t/real-world-coordinates/340782)), camera clipping (`A15`, [forums.sketchup.com](https://forums.sketchup.com/t/clipping-bug-when-in-parallel-projection/119465)), or the 0.001-inch tiny-face failure that forces "The Dave Method" (`A2`, [forums.sketchup.com](https://forums.sketchup.com/t/the-dave-method/49720)).

### D2. Manifold-Preserving Half-Edge Solids with Strict Planarity Constraints (`val3dity`-Clean by Construction)
- **References:** Fixes `A7`, `A8`, `A9`, `A10`, `A11`, `A16`, `B3`, `B4`, `B6`, `B7`, and builds on the solids-first lesson of `C1`.
- **Opportunity:** Enforcing 2-manifold solid invariants, consistent outward face normals (`A16`, [forums.sketchup.com](https://forums.sketchup.com/t/why-does-sketchup-do-this/214828)), and exact face planarity during Push/Pull and Boolean edits prevents non-solid groups (`A7`, [forums.sketchup.com](https://forums.sketchup.com/t/solid-inspector-shows-good-but-it-is-not-solid/206705)), avoids accidental sticky-geometry distortion (`A11`, [forums.sketchup.com](https://forums.sketchup.com/t/beginner-problems-merging-objects-copiyng/134923)), preserves Component linkages across booleans (`A8`, [forums.sketchup.com](https://forums.sketchup.com/t/why-does-solid-tool-turn-component-into-group/228560)), and eliminates `val3dity` errors `203`, `204`, and `302` (`B3`, [val3dity.readthedocs.io](https://val3dity.readthedocs.io/2.6.0/errors/)).

### D3. Built-In Automatic Semantic Classification (`WallSurface`, `RoofSurface`, `GroundSurface`, `Opening`)
- **References:** Fixes `A6`, `B4`, `B5`, `B6`, `B7`, `B9`, and `C4`.
- **Opportunity:** Instead of forcing users to manually paint semantics face-by-face as in `CityEditor` (`B7`, [assets.3dis.de](https://assets.3dis.de/CityEditor_en.pdf)) or write complex FME/CGA rules (`B5`, `B6`, [community.safe.com](https://community.safe.com/data-7/citygml-data-geometry-repair-13300)), the modeler can automatically classify boundary surfaces from surface normal orientation and topological role during footprint extrusion and roof editing, with instant visual semantic inspection like `FZKViewer` (`B8`, [iai.kit.edu](https://www.iai.kit.edu/english/1648.php)). (**ASSUMPTION:** Automatic normal-based classification with interactive override will reduce manual semantic tagging time by >80% compared to face-by-face tagging.)

### D4. Hybrid Interactive + Automated LOD1–LOD2.2 Roof Fitting over Point Clouds and Footprints
- **References:** Bridges `B1` (`3dfier`), `B2` (`roofer` / 3D BAG), and `B7` (`CityEditor`).
- **Opportunity:** Automated pipelines like `3dfier` (`B1`, [tudelft3d.github.io/3dfier](https://tudelft3d.github.io/3dfier/generate_lod1.html)) and `roofer` (`B2`, [innovation.3dbag.nl/roofer](https://innovation.3dbag.nl/roofer/)) reconstruct millions of buildings in batch but provide no interactive editor when complex roofs fail or point clouds are noisy ([docs.3dbag.nl](https://docs.3dbag.nl/en/)). Combining one-click footprint extrusion and parametric roof fitting (flat, gable, hip, mansard, shed) with SketchUp-style ridge/eave push-pull snapping fills the gap between batch CLI pipelines and manual CAD.

### D5. Fast Parametric LOD3 Façade & Opening Templates (Windows, Doors, Balconies, Recesses)
- **References:** Fixes `B5`, `B7`, `B9`, and `A16`.
- **Opportunity:** Academic benchmarks highlight that LOD3 modeling is bottlenecked by extreme manual labor to cut and semantically tag windows, doors, and recesses (`B9`, [tum2t.win](https://tum2t.win/datasets/cm-buildings), [github.com/gdslab/GT-LOD3-Benchmark](https://github.com/gdslab/GT-LOD3-Benchmark)). Providing SketchUp-style array/grid snapping and parametric opening components that automatically cut manifold reveals into `WallSurface` faces without Z-fighting (`A16`, [forums.sketchup.com](https://forums.sketchup.com/t/why-does-sketchup-do-this/214828)) and auto-populate `bldg:Window`/`bldg:Door` semantics directly addresses the hardest step in LOD3 production.

### D6. Native First-Class CityJSON, CityGML, 3D Tiles, and IFC I/O with Integrated `val3dity` Check
- **References:** Fixes `A6`, `B3`, `B4`, `B6`, `B7`, `B8`, and `C4`.
- **Opportunity:** Eliminating the fragile multi-tool chain (GIS -> FME -> SketchUp/Rhino -> `cjio` -> `val3dity`) seen in `B6` and `B9` ([community.safe.com](https://community.safe.com/data-7/citygml-data-geometry-repair-13300), [isprs-archives.copernicus.org](https://isprs-archives.copernicus.org/articles/L-4-W2-2026/95/2026/isprs-archives-L-4-W2-2026-95-2026.pdf)) by embedding native CityJSON/CityGML/IFC I/O and live `val3dity` error highlighting (`B3`, [val3dity.readthedocs.io](https://val3dity.readthedocs.io/2.6.0/errors/)) lets practitioners validate and fix ISO 19107 errors inside the viewport before exporting.

### D7. Multi-Threaded Scene Graph & GPU-Instanced City-Scale Viewport
- **References:** Fixes `A1`, `A13`, `A14`, and `B8`.
- **Opportunity:** SketchUp's single-threaded bottleneck (`A1`, [forums.sketchup.com](https://forums.sketchup.com/t/how-do-you-keep-large-sketchup-models-running-smoothly/350331)) and LayOut's vector rendering freezes (`A13`, [reddit.com](https://www.reddit.com/r/Sketchup/comments/hbcj1s/layout_is_so_damn_slow/)) make editing whole city tiles (tens of thousands of LOD2/LOD3 buildings) impractical. A multi-threaded geometry/command pipeline with GPU instancing for repeated LOD3 façade components and transactional non-blocking undo (`A14`, [forums.sketchup.com](https://forums.sketchup.com/t/how-not-to-freeze-sketchup-when-ruby-is-running/171079/2)) enables smooth district-scale editing.

### D8. Open-Source, Cross-Platform Native Desktop App (macOS, Windows, Linux) with Stable Python Scripting
- **References:** Fixes `A3`, `A4`, `A12`, `B5`, `B6`, `B7`, `B8`, and aligns with community demand shown in `C3`.
- **Opportunity:** Delivering a free, open-source native desktop binary on macOS, Windows, and Linux directly addresses user frustration over SketchUp's subscription lock-in (`A3`, [forums.sketchup.com](https://forums.sketchup.com/t/sketchup-should-bring-back-classic-licences/348946/4)), lack of native Linux support (`A12`, [forums.sketchup.com](https://forums.sketchup.com/t/how-to-use-sketchup-on-linux/131201)), and Windows-only CityGML tools (`B7`, `B8`, [iai.kit.edu](https://www.iai.kit.edu/english/1648.php)), while a versioned Python API avoids SketchUp's recurring Ruby C-extension breakage (`A4`, [forums.sketchup.com](https://forums.sketchup.com/t/important-upcoming-ruby-update/348793)) and fits natively into the Python GIS ecosystem (`cjio`, `GDAL`, `PyPROJ`).

---

## Sources Count

| Section | Sourced Items Count | ASSUMPTION Items Count | Distinct Source URLs Cited |
| :--- | :---: | :---: | :---: |
| **Section A** (SketchUp user pain points) | 17 (`A1`–`A17`) | 0 | 34 |
| **Section B** (LOD1–LOD3 workflows in practice) | 9 (`B1`–`B9`) | 0 | 20 |
| **Section C** (Hew `github.com/hew3d/hew`) | 4 (`C1`–`C4`) | 0 | 7 |
| **Section D** (Differentiation opportunities) | 8 (`D1`–`D8`) | 1 (in `D3`) | 16 (cross-referenced from A–C) |
| **Total** | **38** | **1** | **61 unique URLs** |
