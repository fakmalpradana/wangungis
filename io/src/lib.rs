//! Writing a project to mesh formats other tools read. Hew's writers cover one
//! kernel `Document`; OBJ is ours, as Hew has none and it is the one format here
//! that crosses tiles. Everything is in project-local meters: the project's
//! origin is not added, so that f32 consumers keep their precision.

use std::fmt::Write;

use kernel::{Document, MaterialPalette, NodeId, Transform};
use tessellate::{RenderMesh, TessellateError, tessellate};
use wangungis_scene::project::{Project, node_visible};

/// Facets per full turn for a true curve, in the formats that re-facet.
const SEGMENTS_PER_TURN: u32 = 48;

#[derive(Debug)]
pub enum ExportError {
    /// The file name's extension is none of [`FORMATS`].
    UnknownFormat,
    /// Hew's writers take one kernel tile; this project has more. OBJ can.
    MultiTile,
    NothingToExport,
    Mesh(TessellateError),
    Hew(mesh_export::ExportError),
}

/// Extensions [`export`] takes.
pub const FORMATS: [&str; 5] = ["obj", "stl", "glb", "gltf", "3mf"];

/// `project` as `format`, one of [`FORMATS`] (any case).
pub fn export(project: &Project, format: &str) -> Result<Vec<u8>, ExportError> {
    let format = format.to_ascii_lowercase();
    match format.as_str() {
        "obj" => obj(project),
        "stl" | "glb" | "gltf" | "3mf" => {
            if project
                .tiles
                .values()
                .all(|d| d.top_level_nodes().is_empty())
            {
                return Err(ExportError::NothingToExport);
            }
            let doc = project.single().ok_or(ExportError::MultiTile)?;
            mesh_export::export(doc, &format, SEGMENTS_PER_TURN, false).map_err(ExportError::Hew)
        }
        _ => Err(ExportError::UnknownFormat),
    }
}

/// One tile's visible geometry: each mesh with its pose, if any.
fn tile_meshes(doc: &Document) -> Result<Vec<(RenderMesh, Option<Transform>)>, TessellateError> {
    let palette = MaterialPalette::default();
    let mut out = Vec::new();
    for node in doc.top_level_nodes() {
        if !node_visible(doc, node) {
            continue;
        }
        for id in doc.leaf_objects_under(node) {
            if node_visible(doc, NodeId::Object(id))
                && let Some(obj) = doc.object(id)
            {
                out.push((tessellate(obj, &palette)?, None));
            }
        }
        for inst in doc.leaf_instances_under(node) {
            let (Some(def), Some(pose)) = (doc.instance_def(inst), doc.instance_pose(inst)) else {
                continue;
            };
            if !node_visible(doc, NodeId::Instance(inst)) {
                continue;
            }
            let shown = doc.expanded_def_placements_shown(def);
            for ((member, local), shown) in doc.expanded_def_placements(def).into_iter().zip(shown)
            {
                if shown && let Some(obj) = doc.object(member) {
                    out.push((tessellate(obj, &palette)?, Some(local.then(&pose))));
                }
            }
        }
    }
    Ok(out)
}

/// Wavefront OBJ: triangles, shared vertices per mesh, one object per solid.
fn obj(project: &Project) -> Result<Vec<u8>, ExportError> {
    let mut text = String::from("# WangunGIS\n# meters, Z up, relative to the project origin\n");
    if let Some(o) = project.origin {
        let _ = writeln!(
            text,
            "# origin EPSG:{} ({:.3}, {:.3}, {:.3})",
            o.epsg, o.origin[0], o.origin[1], o.origin[2]
        );
    }
    let mut base = 1usize;
    let mut solid = 0usize;
    for (key, doc) in &project.tiles {
        let c = key.center();
        for (mesh, pose) in tile_meshes(doc).map_err(ExportError::Mesh)? {
            solid += 1;
            let _ = writeln!(text, "o solid_{solid}");
            for p in mesh.positions.chunks(3) {
                let local = kernel::Point3::new(p[0].into(), p[1].into(), p[2].into());
                let q = pose.as_ref().map_or(local, |t| t.apply_point(local));
                let _ = writeln!(text, "v {} {} {}", q.x + c[0], q.y + c[1], q.z + c[2]);
            }
            for t in mesh.indices.chunks(3) {
                let at = |k: usize| base + t[k] as usize;
                let _ = writeln!(text, "f {} {} {}", at(0), at(1), at(2));
            }
            base += mesh.positions.len() / 3;
        }
    }
    if solid == 0 {
        return Err(ExportError::NothingToExport);
    }
    Ok(text.into_bytes())
}
