//! Kernel tiles and the multi-tile project container (ADR-004, ADR-006).
//!
//! One Hew `Document` is only safe within [`KERNEL_FRAME_BOUND_M`] of its own
//! origin, so a city is a [`Project`] of 1 km tiles, one `Document` each, with
//! the document origin at the tile center. Tile (0, 0) is centered on the
//! project origin, so a plain `.hew` is exactly a one-tile project.
//!
//! Every tile carries its own `wangungis` origin (the world position of its
//! center) and key, so a tile opened alone, even in Hew, is georeferenced.
//! Projects of two or more tiles are saved as a byte-stable zip holding
//! [`MANIFEST`] plus `tiles/<i>_<j>.hew`.

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};

use kernel::{
    AttrTarget, AttrValue, Document, EntityRef, InsertOptions, NodeId, Point3, Transform, Vec3,
};
use serde::{Deserialize, Serialize};

use crate::{KERNEL_FRAME_BOUND_M, NS, OriginError, ProjectOrigin, check_local};

/// Edge length (m) of a kernel tile.
pub const TILE_SIZE_M: f64 = 1_000.0;

/// Container entry naming the project. A plain `.hew` (also a zip) has
/// `manifest.json` instead, so the two never collide.
pub const MANIFEST: &str = "wangungis-project.json";

const FORMAT: &str = "wangungis-project";
const VERSION: u32 = 1;
const KEY_TILE: &str = "tile";
/// Transient marker: a moved root was user-hidden (see `copy_to_tile`).
const KEY_HIDDEN: &str = "split-hidden";
/// Manifest origin vs a tile's own stored origin.
const ORIGIN_MATCH_M: f64 = 1e-3;

/// Tile index on the project grid: tile (i, j) is centered on project-local
/// (i·[`TILE_SIZE_M`], j·[`TILE_SIZE_M`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TileKey {
    pub i: i32,
    pub j: i32,
}

impl TileKey {
    pub const ORIGIN: TileKey = TileKey { i: 0, j: 0 };

    /// The tile whose square holds project-local `(x, y)`. Callers keep the
    /// point within the project bound, so the index fits easily.
    pub fn of(xy: [f64; 2]) -> Self {
        TileKey {
            i: (xy[0] / TILE_SIZE_M).round() as i32,
            j: (xy[1] / TILE_SIZE_M).round() as i32,
        }
    }

    /// Project-local position of the tile's document origin.
    pub fn center(self) -> [f64; 3] {
        [
            self.i as f64 * TILE_SIZE_M,
            self.j as f64 * TILE_SIZE_M,
            0.0,
        ]
    }

    fn file(self) -> String {
        format!("tiles/{}_{}.hew", self.i, self.j)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectError {
    Origin(OriginError),
    /// Hew refused a load, copy, insert, or delete.
    Kernel(String),
    /// The container or one of its tiles is not what this crate writes.
    Container(String),
    /// Geometry in `tile` reaches `reach_m` from the tile center, past
    /// [`KERNEL_FRAME_BOUND_M`] (e.g. one object longer than ~1 km).
    TooFar {
        tile: TileKey,
        reach_m: f64,
    },
}

impl From<OriginError> for ProjectError {
    fn from(e: OriginError) -> Self {
        ProjectError::Origin(e)
    }
}

fn kernel<E: std::fmt::Debug>(e: E) -> ProjectError {
    ProjectError::Kernel(format!("{e:?}"))
}

fn container(msg: impl Into<String>) -> ProjectError {
    ProjectError::Container(msg.into())
}

/// A city as kernel tiles. Tile documents hold geometry relative to their
/// own center; [`TileKey::center`] places them in the project.
#[derive(Debug)]
pub struct Project {
    pub origin: Option<ProjectOrigin>,
    pub tiles: BTreeMap<TileKey, Document>,
}

#[derive(Serialize, Deserialize)]
struct Manifest {
    format: String,
    version: u32,
    epsg: Option<u32>,
    origin: Option<[f64; 3]>,
    tile_size_m: u32,
    tiles: Vec<TileEntry>,
}

#[derive(Serialize, Deserialize)]
struct TileEntry {
    i: i32,
    j: i32,
    file: String,
}

/// Every world-space vertex under `node`: its world objects, plus every
/// instance's definition geometry through the instance pose (the same walk
/// Hew uses for its own placement anchor).
fn node_points(doc: &Document, node: NodeId) -> Vec<Point3> {
    let verts = |o: kernel::ObjectId| {
        doc.object(o)
            .into_iter()
            .flat_map(|o| o.vertices().values().map(|v| v.position))
    };
    let mut points: Vec<Point3> = doc
        .leaf_objects_under(node)
        .into_iter()
        .flat_map(verts)
        .collect();
    for inst in doc.leaf_instances_under(node) {
        let (Some(def), Some(pose)) = (doc.instance_def(inst), doc.instance_pose(inst)) else {
            continue;
        };
        for (member, local) in doc.expanded_def_placements(def) {
            let t = local.then(&pose);
            points.extend(verts(member).map(|p| t.apply_point(p)));
        }
    }
    points
}

/// Largest horizontal distance (m) of any world-space vertex (objects and
/// instance geometry) from the document origin.
pub fn horizontal_reach(doc: &Document) -> f64 {
    doc.top_level_nodes()
        .into_iter()
        .flat_map(|n| node_points(doc, n))
        .map(|p| p.x.hypot(p.y))
        .fold(0.0, f64::max)
}

/// Where a top-level node belongs: the center of its points' XY bounds.
/// `None` for a node with no points (an empty group).
fn anchor(points: &[Point3]) -> Option<[f64; 2]> {
    let (first, rest) = points.split_first()?;
    let (lo, hi) = rest
        .iter()
        .fold(([first.x, first.y], [first.x, first.y]), |(lo, hi), p| {
            (
                [lo[0].min(p.x), lo[1].min(p.y)],
                [hi[0].max(p.x), hi[1].max(p.y)],
            )
        });
    Some([(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0])
}

fn entity(node: NodeId) -> AttrTarget {
    AttrTarget::Entity(match node {
        NodeId::Object(id) => EntityRef::Object(id),
        NodeId::Group(id) => EntityRef::Group(id),
        NodeId::Instance(id) => EntityRef::Instance(id),
    })
}

/// Copies `nodes` into a new document whose origin is `center`, as a pure
/// translation. With the drawing axes at `center` Hew's `extract_item`
/// re-expresses every node through them and skips its bottom-center
/// re-origin, which would leave visibly hidden instances behind.
fn copy_to_tile(
    doc: &mut Document,
    nodes: &[NodeId],
    center: [f64; 3],
) -> Result<Document, ProjectError> {
    doc.set_axes(
        Point3::new(center[0], center[1], center[2]),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    )
    .map_err(kernel)?;
    // extract_item makes the item's roots visible; a marker attribute (which
    // is copied) carries the user-hidden state across.
    for &n in nodes {
        if doc.node_user_hidden(n) {
            doc.attr_set(entity(n), NS, KEY_HIDDEN, AttrValue::Bool(true))
                .map_err(kernel)?;
        }
    }
    let item = doc.extract_item(nodes, false).map_err(kernel)?;
    let mut tile = Document::new();
    tile.insert_document(
        &item,
        &InsertOptions {
            pose: Transform::IDENTITY,
            provenance: None,
        },
    )
    .map_err(kernel)?;
    for root in tile.top_level_nodes() {
        let dict = tile.attr_get(&entity(root)).map_err(kernel)?;
        if dict
            .as_ref()
            .and_then(|d| d.get(NS))
            .and_then(|ns| ns.get(KEY_HIDDEN))
            .is_some()
        {
            tile.attr_delete(entity(root), NS, Some(KEY_HIDDEN))
                .map_err(kernel)?;
            tile.set_node_user_hidden(root, true);
        }
    }
    Ok(tile)
}

impl Project {
    /// A project from one document. Within [`KERNEL_FRAME_BOUND_M`] it stays a
    /// single tile, untouched apart from tile metadata. Otherwise every
    /// top-level node moves, losslessly (Hew extract + insert as a pure
    /// translation; attributes, hidden state and tags included), to the tile
    /// holding its anchor. Tile (0, 0) keeps the source document itself, so
    /// world sketches, annotations, guides, scenes and styles stay there.
    /// Every point must lie within the project bound.
    pub fn from_document(mut doc: Document) -> Result<Self, ProjectError> {
        let origin = ProjectOrigin::read(&doc)?;
        let mut moves: BTreeMap<TileKey, Vec<NodeId>> = BTreeMap::new();
        let mut reach = 0.0f64;
        for node in doc.top_level_nodes() {
            let points = node_points(&doc, node);
            for p in &points {
                check_local([p.x, p.y, p.z])?;
                reach = reach.max(p.x.hypot(p.y));
            }
            if let Some(key) = anchor(&points).map(TileKey::of)
                && key != TileKey::ORIGIN
            {
                moves.entry(key).or_default().push(node);
            }
        }
        let mut tiles = BTreeMap::new();
        let split = reach > KERNEL_FRAME_BOUND_M;
        if split {
            let axes = doc.axes();
            for (key, nodes) in &moves {
                tiles.insert(*key, copy_to_tile(&mut doc, nodes, key.center())?);
            }
            doc.set_axes(axes.origin, axes.x, axes.y).map_err(kernel)?;
            let moved: Vec<NodeId> = moves.into_values().flatten().collect();
            doc.delete_selection(&moved).map_err(kernel)?;
        }
        tiles.insert(TileKey::ORIGIN, doc);
        for (key, tile) in &mut tiles {
            write_tile_meta(tile, *key, origin)?;
            if split {
                // Drops the split's undo steps: undoing the delete in tile
                // (0, 0) would duplicate every moved node.
                *tile = Document::load(&tile.save()).map_err(kernel)?;
            }
        }
        let p = Project { origin, tiles };
        p.check_reach()?;
        Ok(p)
    }

    /// True when `bytes` are a project container rather than a plain `.hew`.
    pub fn is_container(bytes: &[u8]) -> bool {
        zip::ZipArchive::new(Cursor::new(bytes)).is_ok_and(|z| z.index_for_name(MANIFEST).is_some())
    }

    /// Opens a project container, or a plain `.hew` through
    /// [`Project::from_document`].
    pub fn open(bytes: &[u8]) -> Result<Self, ProjectError> {
        match zip::ZipArchive::new(Cursor::new(bytes)) {
            Ok(mut zip) if zip.index_for_name(MANIFEST).is_some() => {
                Self::load(&mut zip, bytes.len() as u64)
            }
            _ => Self::from_document(Document::load(bytes).map_err(kernel)?),
        }
    }

    /// The one document of a single-tile project, which saves as plain `.hew`.
    pub fn single(&self) -> Option<&Document> {
        match (self.tiles.len(), self.tiles.get(&TileKey::ORIGIN)) {
            (1, Some(doc)) => Some(doc),
            _ => None,
        }
    }

    /// Plain `.hew` bytes for a single-tile project, else the container.
    /// Byte-stable: the same project saves to the same bytes.
    pub fn save(&self) -> Vec<u8> {
        if let Some(doc) = self.single() {
            return doc.save();
        }
        let manifest = Manifest {
            format: FORMAT.into(),
            version: VERSION,
            epsg: self.origin.map(|o| o.epsg),
            origin: self.origin.map(|o| o.origin),
            tile_size_m: TILE_SIZE_M as u32,
            tiles: self
                .tiles
                .keys()
                .map(|k| TileEntry {
                    i: k.i,
                    j: k.j,
                    file: k.file(),
                })
                .collect(),
        };
        // Same entry options as Hew's own byte-stable `.hew` writer.
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .last_modified_time(zip::DateTime::default());
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let mut add = |name: &str, bytes: &[u8]| {
            zip.start_file(name, opts).expect("in-memory zip entry");
            zip.write_all(bytes).expect("in-memory zip write");
        };
        add(
            MANIFEST,
            &serde_json::to_vec_pretty(&manifest).expect("manifest serializes"),
        );
        for (key, doc) in &self.tiles {
            add(&key.file(), &doc.save());
        }
        zip.finish().expect("in-memory zip").into_inner()
    }

    /// `budget`: total bytes all entries may yield. Entries are stored, not
    /// compressed, so an honest container never reads more than its own size;
    /// overlapping or oversized entries are refused.
    fn load(
        zip: &mut zip::ZipArchive<Cursor<&[u8]>>,
        mut budget: u64,
    ) -> Result<Self, ProjectError> {
        let m: Manifest = serde_json::from_slice(&read_entry(zip, MANIFEST, &mut budget)?)
            .map_err(|e| container(format!("{MANIFEST}: {e}")))?;
        if m.format != FORMAT || m.version != VERSION {
            return Err(container(format!(
                "unsupported project {:?} version {}",
                m.format, m.version
            )));
        }
        if m.tile_size_m != TILE_SIZE_M as u32 {
            return Err(container(format!("tile size {} m", m.tile_size_m)));
        }
        let origin = match (m.epsg, m.origin) {
            (None, None) => None,
            (Some(epsg), Some(origin)) => {
                let o = ProjectOrigin { epsg, origin };
                // Validates finiteness and the EPSG code the same way a write does.
                o.write(&mut Document::new())?;
                Some(o)
            }
            _ => return Err(container("epsg and origin must both be present")),
        };
        if m.tiles.is_empty() {
            return Err(container("no tiles"));
        }
        if m.tiles.len() == 1 && (m.tiles[0].i, m.tiles[0].j) == (0, 0) {
            // `save` writes such a project as a plain .hew.
            return Err(container("a single-tile project must be a plain .hew"));
        }
        if zip.len() != m.tiles.len() + 1 {
            return Err(container(format!(
                "{} entries for {} tiles",
                zip.len(),
                m.tiles.len()
            )));
        }
        let mut tiles = BTreeMap::new();
        for e in &m.tiles {
            let key = TileKey { i: e.i, j: e.j };
            check_local(key.center())?;
            if e.file != key.file() {
                return Err(container(format!("tile {key:?} stored as {:?}", e.file)));
            }
            let doc = Document::load(&read_entry(zip, &e.file, &mut budget)?).map_err(kernel)?;
            check_tile_meta(&doc, key, origin)?;
            if tiles.insert(key, doc).is_some() {
                return Err(container(format!("tile {key:?} listed twice")));
            }
        }
        let p = Project { origin, tiles };
        p.check_reach()?;
        Ok(p)
    }

    fn check_reach(&self) -> Result<(), ProjectError> {
        for (key, doc) in &self.tiles {
            let reach_m = horizontal_reach(doc);
            if reach_m > KERNEL_FRAME_BOUND_M {
                return Err(ProjectError::TooFar {
                    tile: *key,
                    reach_m,
                });
            }
        }
        Ok(())
    }
}

fn read_entry(
    zip: &mut zip::ZipArchive<Cursor<&[u8]>>,
    name: &str,
    budget: &mut u64,
) -> Result<Vec<u8>, ProjectError> {
    let entry = zip
        .by_name(name)
        .map_err(|e| container(format!("{name}: {e}")))?;
    let mut buf = Vec::new();
    entry
        .take(*budget + 1)
        .read_to_end(&mut buf)
        .map_err(|e| container(format!("{name}: {e}")))?;
    *budget = budget
        .checked_sub(buf.len() as u64)
        .ok_or_else(|| container(format!("{name}: entries exceed the container size")))?;
    Ok(buf)
}

/// The tile's own origin: the world position of its center.
fn tile_origin(project: ProjectOrigin, key: TileKey) -> ProjectOrigin {
    let c = key.center();
    ProjectOrigin {
        epsg: project.epsg,
        origin: std::array::from_fn(|k| project.origin[k] + c[k]),
    }
}

fn write_tile_meta(
    doc: &mut Document,
    key: TileKey,
    origin: Option<ProjectOrigin>,
) -> Result<(), ProjectError> {
    let ij = AttrValue::List(vec![
        AttrValue::Int(key.i.into()),
        AttrValue::Int(key.j.into()),
    ]);
    doc.attr_set(AttrTarget::Document, NS, KEY_TILE, ij)
        .map_err(kernel)?;
    if let Some(o) = origin {
        tile_origin(o, key).write(doc)?;
    }
    Ok(())
}

fn check_tile_meta(
    doc: &Document,
    key: TileKey,
    origin: Option<ProjectOrigin>,
) -> Result<(), ProjectError> {
    let dict = doc.attr_get(&AttrTarget::Document).map_err(kernel)?;
    let stored = dict
        .as_ref()
        .and_then(|d| d.get(NS))
        .and_then(|ns| ns.get(KEY_TILE));
    let want = AttrValue::List(vec![
        AttrValue::Int(key.i.into()),
        AttrValue::Int(key.j.into()),
    ]);
    if stored != Some(&want) {
        return Err(container(format!("tile {key:?}: stored key {stored:?}")));
    }
    match (
        ProjectOrigin::read(doc)?,
        origin.map(|o| tile_origin(o, key)),
    ) {
        (None, None) => Ok(()),
        (Some(got), Some(want))
            if got.epsg == want.epsg
                && (0..3).all(|k| (got.origin[k] - want.origin[k]).abs() <= ORIGIN_MATCH_M) =>
        {
            Ok(())
        }
        (got, want) => Err(container(format!(
            "tile {key:?}: origin {got:?}, manifest implies {want:?}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_keys_round_to_the_nearest_center() {
        assert_eq!(TileKey::of([0.0, 0.0]), TileKey::ORIGIN);
        assert_eq!(TileKey::of([499.9, -499.9]), TileKey::ORIGIN);
        assert_eq!(TileKey::of([500.1, -1_700.0]), TileKey { i: 1, j: -2 });
        assert_eq!(TileKey { i: -3, j: 7 }.center(), [-3_000.0, 7_000.0, 0.0]);
    }
}
