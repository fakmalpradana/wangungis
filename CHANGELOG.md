# Changelog

All notable changes to this project are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project aims to follow [Semantic Versioning](https://semver.org/).

## [0.1.0] - unreleased

First pre-release for beta testing.

### Added
- Drawing tools: Line, Rectangle, Circle, Polygon, Arc, with typed dimensions and angles.
- Editing tools: Push/Pull (several objects at once, snap to existing heights, new-solid mode), Offset, Move, Rotate, Scale, Eraser, Follow Me.
- Copy during Move and Rotate, with copy counts (`x5`, `/5`).
- Guide lines (tape measure).
- Inference and snapping: endpoint, midpoint, edge, face, guide, parallel and perpendicular, with axis lock and labels.
- Groups and components, editable in place, with undo across open and close.
- Tags with visibility toggles, and an outliner panel.
- Paint bucket with solid colors, saved in the file and exported to `.glb`.
- Display units (m, mm, ft), remembered between sessions.
- LAS point cloud import with a progress window and cache, snapping to points, per-cloud point size, translation to align, and a slice box.
- Native `.hew` file format; export to `.obj`, `.stl`, `.glb` and `.3mf`.
- OS file dialogs, unsaved-changes prompts, and autosave with crash recovery.
- 16 UI languages (14 are machine-translated drafts), with embedded Noto fonts.
- Toolbar icons, standard view buttons (Top, Front, Right, Iso) and Zoom extents.
- CI workflow building release binaries for Linux, macOS (Apple Silicon) and Windows.

### Known limitations
See the "Known limitations" section of the [README](README.md).
