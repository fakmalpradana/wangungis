# Contributing

Thanks for helping with WangunGIS. The full project rules are in [AGENTS.md](AGENTS.md) (Indonesian). This page is the English summary for human contributors. Where the two disagree, AGENTS.md wins.

## Getting started

```bash
git clone https://github.com/fakmalpradana/wangungis.git
cd wangungis
cargo build --workspace          # the toolchain (Rust 1.97.1) installs itself from rust-toolchain.toml
cargo run -p wangungis           # the app
```

The first build fetches the pinned Hew crates from Git and takes a while.

Read the `SPEC.md` of the crate you work in before changing it. Each one states the crate's purpose, public API, invariants and non-goals.

| Crate | What it does |
|---|---|
| `scene` | Project origin (EPSG + `f64` point), tiles, safe coordinate range for the kernel |
| `render` | wgpu renderer, per-tile batching, relative-to-center (RTC) rendering, golden-image tests |
| `tools` | Editing layer without GPU or UI: tools, snapping, picking, groups, tags, paint, undo/redo |
| `io` | Mesh export (`.obj`, `.stl`, `.glb`, `.3mf`) |
| `pointcloud` | LAS import, cache, slice box |
| `i18n` | UI text: `t()`, `tf()`, the 16 catalogs in `locales/` |
| `ui` | The eframe app: toolbar, icons, value input box, menus, fonts; binary `wangungis` |

## Picking a task

Take an issue labelled `help wanted` or `good first issue`, and comment on it before you start so two people do not work on the same thing. For anything not in an issue, open one first. [ROADMAP.md](ROADMAP.md) shows where the project is going.

## Rules that matter most

1. **Correct before fast.** Wrong geometry breaks everything downstream.
2. **Every behavior has an automated test.** A feature without a test does not exist. New geometry operations also need a property-based test (random input, invariants hold). Never delete or weaken a test to make CI green.
3. **Reuse before writing.** Half-edge mesh, Push/Pull, booleans, undo and basic inference come from the Hew crates. Do not rewrite them; wrap them, or propose a fix upstream.
4. **Geometry rules.** World coordinates are `f64`. Do not compare floats with `==` or ad hoc epsilons: tolerances live in one place and are documented. Mesh operations must keep the mesh manifold, consistently oriented and free of degenerate faces.
5. **Ask the owner first** before you add a dependency (it needs a short ADR in `docs/adr/`: license, size, activity, alternatives), change the Hew pin, change the native file format, change a public API or a geometry tolerance, or delete a test.
6. **UI text goes through `i18n`.** Use `t("group.name")` or `tf("group.name", &[..])`, never a string literal. Add a new key to all 16 files in `i18n/locales/` with the same `{n}` placeholders. `id.json` defines the key set. For languages other than Indonesian and English, follow `i18n/GLOSSARY.md`.
7. **Clean-room.** Copy workflows and UX ideas from other modelers, never their code, assets, icons, names or trademarks. No proprietary or paid dependencies.
8. **Licensing.** The project is AGPL-3.0-only. By contributing you agree your contribution is licensed under it.

## Before you open a pull request

Run the same gate as CI:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
WANGUNGIS_REQUIRE_GPU=1 cargo test --workspace --locked
WANGUNGIS_REQUIRE_GPU=1 cargo test --workspace --locked --release
```

Without `WANGUNGIS_REQUIRE_GPU=1`, the golden-image and UI smoke tests are skipped silently when no GPU is found. On a machine without a GPU, leave the variable out and say so in the PR.

Then:
- Branch from `main` as `feat/<crate>-<topic>`, `fix/...`, `refactor/...` or `docs/...`. Nobody pushes to `main` directly.
- Small commits in [Conventional Commits](https://www.conventionalcommits.org) form, for example `feat(tools): mirror selection`.
- If you changed a public API, update that crate's `SPEC.md` in the same PR.
- Note user-visible changes in `CHANGELOG.md` under the next version.
- The PR description has 3–5 lines of summary, the risks, the tests you added and, where relevant, measurements. CI on macOS, Windows and Linux must be green.

## Review

Every PR gets a correctness review: invariants, edge cases and precision, not only style. The author is never the only reviewer. Some reviews and some code in this project come from AI agents; their output gets the same gate and the same human sign-off as anyone's. The owner merges.

## Language

Code, comments, identifiers, commit messages and PRs are in English. Planning documents, ADRs and user documentation are mostly in Indonesian; ask in the issue if something is unclear.
