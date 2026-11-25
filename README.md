# WangunGIS

WangunGIS is an open-source desktop 3D modeler for building models at LOD1 to LOD3. It follows a SketchUp-like workflow: draw faces, then Push/Pull them into solids. It runs on Windows, macOS and Linux from one Rust codebase (egui/eframe for the UI, wgpu for rendering). LAS point clouds can be imported and used as a reference to model on top of.

**Status: pre-release / beta (v0.1.0, not yet published).** Expect rough edges, and keep backups of anything that matters.

## Features

**Drawing and editing tools**
- Line, Rectangle, Circle, Polygon, Arc
- Push/Pull (including several objects at once, snapping to existing heights, and a "new solid" mode), Offset, Move, Rotate, Scale, Eraser, Follow Me
- Copy while moving or rotating, with typed copy counts (`x5`, `/5`)
- Guide lines from the tape tool

**Groups, components, tags, outliner, paint**
- Groups and components (copies of a component share geometry), editable in place
- Tags with visibility toggles
- Outliner panel listing every object, group and component
- Paint bucket with solid colors (colors are saved and exported to `.glb`)

**Inference and snapping**
- Endpoint, midpoint, edge, face, guide, parallel and perpendicular snaps, with on-screen labels
- Axis lock with the arrow keys, angle input such as `30d`

**Units**
- Typed values with units (`5;4`, `250cm`, `1500mm`, `10ft`); display unit selectable (m, mm, ft) and remembered

**Point clouds**
- Import `.las` with a progress window, cached for fast re-import
- Snap to points while drawing, per-cloud point size, translate the cloud to align it
- Slice box (clip to an area) to isolate one building

**Files**
- Native `.hew` format; export to `.obj`, `.stl`, `.glb`, `.3mf`
- Autosave every 60 seconds with crash recovery, and prompts before discarding unsaved changes

**Languages**
- 16 UI languages. Indonesian and English are maintained; the other 14 are machine-translated drafts, marked with `*` in the language menu.

## Download and install

Download the archive for your OS from the [GitHub Releases](https://github.com/fakmalpradana/wangungis/releases) page. Archives are named `wangungis-<commit>-<os>.tar.gz` (Linux, macOS) or `.zip` (Windows), each with a matching `.sha256` file. Builds are unsigned.

**Windows (x86_64).** Extract the zip and run `wangungis.exe`. If Windows shows "Windows protected your PC", choose **More info**, then **Run anyway**. If you get an error about `VCRUNTIME140.dll`, install the Microsoft Visual C++ Redistributable (x64).

**macOS (Apple Silicon only).** There is no Intel build yet. The binary is not notarized, so remove the quarantine flag, or right-click it and choose **Open**:

```bash
tar -xzf wangungis-<commit>-macos-arm64.tar.gz
xattr -dr com.apple.quarantine wangungis-<commit>-macos-arm64/wangungis
./wangungis-<commit>-macos-arm64/wangungis
```

**Linux (x86_64).** Needs a Vulkan driver (for example `mesa-vulkan-drivers`). File dialogs need `xdg-desktop-portal` or `zenity`.

```bash
tar -xzf wangungis-<commit>-linux-x86_64.tar.gz
./wangungis-<commit>-linux-x86_64/wangungis
```

**Verify the download** (run in the folder holding the archive and its `.sha256`):

```bash
sha256sum -c wangungis-<commit>-linux-x86_64.tar.gz.sha256    # Linux
shasum -a 256 -c wangungis-<commit>-macos-arm64.tar.gz.sha256 # macOS
```

## Build from source

Install [Rust via rustup](https://rustup.rs). The right compiler version is installed automatically from `rust-toolchain.toml`.

```bash
cargo build --release -p wangungis --locked
./target/release/wangungis [file.hew]
```

## Known limitations

- Push/Pull preview is an outline, not a solid.
- File dialogs on Windows and Linux have not been tested by a human yet.
- Point clouds: `.las` only (no `.laz`), no reprojection (a different EPSG is rejected), no per-class filtering, and the slice box is axis-aligned. Moving a cloud is translation only. The cache in `~/.wangungis/cache/` is not cleaned automatically.
- 14 of the 16 UI languages are unreviewed machine drafts. Arabic is not laid out right-to-left.
- While a component is open for editing, its other copies are hidden.
- You cannot draw on a plane that has geometry on the other side of a group boundary.
- Paint is solid color only (no textures or transparency). Tags have no colors.
- User documentation is Indonesian only.

The full list is in [docs/user/uji-asap.md](docs/user/uji-asap.md) (Indonesian).

## Documentation

- [docs/user/](docs/user/): walkthrough, beta test guide and feedback format. These are written in **Indonesian**.
- [docs/adr/](docs/adr/): architecture decision records (language, UI, rendering, kernel, license, file format and more).

## Contributing

Project rules for contributors and AI agents are in [AGENTS.md](AGENTS.md) (Indonesian). In short: every behavior needs automated tests, geometry correctness comes before speed, and new dependencies need an ADR. The project is clean-room: no code, assets or branding from proprietary modelers.

## License

[AGPL-3.0-only](LICENSE). The bundled Noto fonts are licensed under the SIL Open Font License 1.1 (see `ui/fonts/OFL-*.txt`).
