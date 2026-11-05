//! The clouds of one project: what is in memory, what each view needs, and which
//! point lies under a ray. A thread reads the cache, so a view never waits for disk.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};

use crate::{ChunkInfo, CloudFile, CloudInfo, CloudPoint};

/// Points on screen per square of the point's size. Above 1, as points overlap.
const DENSITY: f64 = 4.0;
/// Points of a chunk kept in memory at most, whatever the budget. Well under
/// wgpu's default buffer limit of 256 MB.
pub const MAX_CHUNK_POINTS: u64 = 1_000_000;
pub const MIN_POINT_SIZE: f32 = 1.0;
pub const MAX_POINT_SIZE: f32 = 8.0;
pub const MIN_DRAW_BUDGET: u64 = 200_000;
pub const MAX_DRAW_BUDGET: u64 = 5_000_000;
/// Fewest points worth asking the disk for.
const MIN_READ: u64 = 8_192;
const MAX_IN_FLIGHT: usize = 16;
/// Points taken in per frame, so that uploading never stalls one.
const POINTS_PER_POLL: usize = 1_000_000;
/// Among points in the cone, those this far behind the nearest count as the same
/// surface, as a share of the distance. The one closest to the ray wins among them.
const SAME_SURFACE: f64 = 0.01;

/// Project-local box. Points outside it are hidden.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Slice {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

impl Slice {
    pub fn contains(&self, p: [f64; 3]) -> bool {
        (0..3).all(|k| p[k] >= self.min[k] && p[k] <= self.max[k])
    }

    fn overlaps(&self, min: [f64; 3], size: [f64; 3]) -> bool {
        (0..3).all(|k| min[k] <= self.max[k] && min[k] + size[k] >= self.min[k])
    }
}

/// What a frame looks through.
#[derive(Debug, Clone, Copy)]
pub struct View {
    pub eye: [f64; 3],
    /// With the eye at the origin, column-major.
    pub view_proj: [[f32; 4]; 4],
    /// Pixels per unit of size at distance 1.
    pub focal_px: f64,
    /// A point's edge in those pixels.
    pub point_px: f64,
}

#[derive(Debug)]
struct Chunk {
    info: ChunkInfo,
    points: Vec<CloudPoint>,
    /// How many this view draws.
    draw: u64,
    /// How many to hold: at least what is drawn, and never less than the floor.
    wanted: u64,
    /// The renderer's copy is out of date.
    dirty: bool,
    pending: bool,
    failed: bool,
}

#[derive(Debug)]
pub struct Cloud {
    pub id: u32,
    pub source: PathBuf,
    pub info: CloudInfo,
    pub visible: bool,
    chunks: Vec<Chunk>,
}

impl Cloud {
    /// Points of this cloud in memory now.
    pub fn held_points(&self) -> u64 {
        self.chunks.iter().map(|c| c.points.len() as u64).sum()
    }

    pub fn name(&self) -> String {
        self.source
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
    }
}

/// A project's clouds as its companion file keeps them (`<project>.pointcloud.json`).
/// Apart from the project file, whose format stays as it is.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SavedClouds {
    pub frame: [f64; 3],
    pub clouds: Vec<SavedCloud>,
    pub point_size: f32,
    pub draw_budget: u64,
    pub snap: bool,
    pub slice: Option<Slice>,
    pub slice_on: bool,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SavedCloud {
    /// Relative to the project's folder when it lies inside it.
    pub path: PathBuf,
    pub visible: bool,
}

/// The companion file of a project at `project`.
pub fn saved_clouds_path(project: &std::path::Path) -> PathBuf {
    let mut name = project.as_os_str().to_owned();
    name.push(".pointcloud.json");
    PathBuf::from(name)
}

enum Job {
    Add(u32, Box<CloudFile>),
    Read {
        id: u32,
        chunk: usize,
        start: u64,
        count: u64,
    },
    Remove(u32),
}

struct Answer {
    id: u32,
    chunk: usize,
    start: u64,
    asked: u64,
    points: Result<Vec<CloudPoint>, String>,
}

/// One chunk's turn in a frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrawChunk {
    pub id: u32,
    pub chunk: usize,
    pub count: u32,
}

/// A chunk whose points changed, to be uploaded again.
#[derive(Debug)]
pub struct ChangedChunk<'a> {
    pub id: u32,
    pub chunk: usize,
    /// Project-local.
    pub min: [f64; 3],
    pub size: [f64; 3],
    pub points: &'a [CloudPoint],
}

pub struct CloudSet {
    clouds: Vec<Cloud>,
    /// World position of the project's zero. Set by the first cloud of a project
    /// without an origin.
    pub frame: [f64; 3],
    /// Most points drawn in a frame.
    pub draw_budget: u64,
    /// Most points in memory.
    pub load_budget: u64,
    /// Edge of a drawn point, in screen points.
    pub point_size: f32,
    /// Points can be snapped to.
    pub snap: bool,
    /// The box last drawn, kept while it is off.
    pub slice: Option<Slice>,
    pub slice_on: bool,
    jobs: Option<Sender<Job>>,
    answers: Option<Receiver<Answer>>,
    in_flight: usize,
    /// Points those reads will bring, already counted against the budget.
    in_flight_points: u64,
    next_id: u32,
    /// Ids the renderer should drop.
    removed: Vec<u32>,
    /// What went wrong reading, for the status bar.
    pub error: Option<String>,
    draws: Vec<DrawChunk>,
}

impl Default for CloudSet {
    fn default() -> Self {
        Self {
            clouds: Vec::new(),
            frame: [0.0; 3],
            draw_budget: 2_000_000,
            load_budget: 5_000_000,
            point_size: 2.0,
            snap: true,
            slice: None,
            slice_on: false,
            jobs: None,
            answers: None,
            in_flight: 0,
            in_flight_points: 0,
            next_id: 0,
            removed: Vec::new(),
            error: None,
            draws: Vec::new(),
        }
    }
}

impl std::fmt::Debug for CloudSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CloudSet")
            .field("clouds", &self.clouds.len())
            .finish_non_exhaustive()
    }
}

fn reader_thread(jobs: Receiver<Job>, answers: Sender<Answer>) {
    let mut files = std::collections::HashMap::new();
    for job in jobs {
        match job {
            Job::Add(id, file) => {
                files.insert(id, file);
            }
            Job::Remove(id) => {
                files.remove(&id);
            }
            Job::Read {
                id,
                chunk,
                start,
                count,
            } => {
                let points = match files.get_mut(&id) {
                    Some(file) => file.read(chunk, start, count).map_err(|e| e.to_string()),
                    None => Ok(Vec::new()),
                };
                let answer = Answer {
                    id,
                    chunk,
                    start,
                    asked: count,
                    points,
                };
                if answers.send(answer).is_err() {
                    return;
                }
            }
        }
    }
}

impl CloudSet {
    pub fn clouds(&self) -> &[Cloud] {
        &self.clouds
    }

    pub fn is_empty(&self) -> bool {
        self.clouds.is_empty()
    }

    /// Starts showing an opened cache. Its points arrive over the next frames.
    pub fn add(&mut self, file: CloudFile, source: PathBuf) -> u32 {
        if self.jobs.is_none() {
            let (jobs, job_rx) = channel();
            let (answer_tx, answers) = channel();
            std::thread::spawn(move || reader_thread(job_rx, answer_tx));
            self.jobs = Some(jobs);
            self.answers = Some(answers);
        }
        let id = self.next_id;
        self.next_id += 1;
        let info = file.info.clone();
        let chunks = info
            .chunks
            .iter()
            .map(|c| Chunk {
                info: *c,
                points: Vec::new(),
                draw: 0,
                wanted: 0,
                dirty: false,
                pending: false,
                failed: false,
            })
            .collect();
        self.send(Job::Add(id, Box::new(file)));
        self.clouds.push(Cloud {
            id,
            source,
            info,
            visible: true,
            chunks,
        });
        id
    }

    pub fn remove(&mut self, id: u32) {
        self.clouds.retain(|c| c.id != id);
        self.removed.push(id);
        self.send(Job::Remove(id));
    }

    /// Ids dropped since the last call, for the renderer.
    pub fn take_removed(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.removed)
    }

    pub fn set_visible(&mut self, id: u32, visible: bool) {
        if let Some(c) = self.clouds.iter_mut().find(|c| c.id == id) {
            c.visible = visible;
        }
    }

    fn send(&self, job: Job) {
        if let Some(jobs) = &self.jobs {
            // A thread gone took its files with it: nothing more will arrive.
            let _ = jobs.send(job);
        }
    }

    /// This set as a companion file beside a project in `dir`.
    pub fn saved(&self, dir: &std::path::Path) -> SavedClouds {
        SavedClouds {
            frame: self.frame,
            clouds: self
                .clouds
                .iter()
                .map(|c| SavedCloud {
                    path: c
                        .source
                        .strip_prefix(dir)
                        .map_or_else(|_| c.source.clone(), std::path::Path::to_owned),
                    visible: c.visible,
                })
                .collect(),
            point_size: self.point_size,
            draw_budget: self.draw_budget,
            snap: self.snap,
            slice: self.slice,
            slice_on: self.slice_on,
        }
    }

    /// Takes the settings of a companion file. The clouds themselves load apart.
    pub fn apply_saved(&mut self, saved: &SavedClouds) {
        self.frame = saved.frame.map(|v| if v.is_finite() { v } else { 0.0 });
        self.point_size = saved.point_size.clamp(MIN_POINT_SIZE, MAX_POINT_SIZE);
        self.draw_budget = saved.draw_budget.clamp(MIN_DRAW_BUDGET, MAX_DRAW_BUDGET);
        self.snap = saved.snap;
        self.slice = saved.slice.filter(|s| {
            (0..3).all(|k| s.min[k].is_finite() && s.max[k].is_finite() && s.min[k] < s.max[k])
        });
        self.slice_on = saved.slice_on && self.slice.is_some();
    }

    /// The box in force, if any.
    pub fn active_slice(&self) -> Option<Slice> {
        self.slice.filter(|_| self.slice_on)
    }

    /// Points or reads are on the way: another frame will change the picture.
    pub fn busy(&self) -> bool {
        self.in_flight > 0
    }

    /// Project-local bounds of what the clouds hold.
    pub fn bounds(&self) -> Option<([f64; 3], [f64; 3])> {
        let mut out = None;
        for c in self.clouds.iter().filter(|c| c.visible) {
            let lo: [f64; 3] = std::array::from_fn(|k| c.info.min[k] - self.frame[k]);
            let hi: [f64; 3] = std::array::from_fn(|k| c.info.max[k] - self.frame[k]);
            out = Some(match out {
                None => (lo, hi),
                Some((a, b)) => {
                    let (a, b): ([f64; 3], [f64; 3]) = (a, b);
                    (
                        std::array::from_fn(|k| a[k].min(lo[k])),
                        std::array::from_fn(|k| b[k].max(hi[k])),
                    )
                }
            });
        }
        out
    }

    /// Takes in what the reader has finished, within what one frame should do.
    pub fn poll(&mut self) {
        let Some(answers) = &self.answers else {
            return;
        };
        let mut taken = 0;
        while taken < POINTS_PER_POLL {
            let Ok(a) = answers.try_recv() else {
                break;
            };
            self.in_flight -= 1;
            self.in_flight_points -= a.asked;
            let chunk = self
                .clouds
                .iter_mut()
                .find(|c| c.id == a.id)
                .and_then(|c| c.chunks.get_mut(a.chunk));
            let Some(chunk) = chunk else {
                continue;
            };
            chunk.pending = false;
            match a.points {
                // Trimmed while the read was under way: not the next points any more.
                Ok(points) if a.start == chunk.points.len() as u64 => {
                    taken += points.len();
                    if !points.is_empty() {
                        chunk.points.extend_from_slice(&points);
                        chunk.dirty = true;
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    chunk.failed = true;
                    self.error = Some(e);
                }
            }
        }
    }

    /// Decides what this view draws and asks the disk for what it lacks.
    pub fn plan(&mut self, view: &View) {
        let slice = self.active_slice();
        let frame = self.frame;
        let px = view.point_px.max(1.0);
        let total_chunks: usize = self.clouds.iter().map(|c| c.chunks.len()).sum();
        // Every chunk keeps a little, so that any view starts with something.
        let floor = (self.load_budget / 10).min(self.draw_budget / 2);
        let base = (floor / total_chunks.max(1) as u64).clamp(256, 65_536);

        let mut sum = 0u64;
        let mut seen: Vec<(f64, usize, usize)> = Vec::new();
        for (i, cloud) in self.clouds.iter_mut().enumerate() {
            for (j, chunk) in cloud.chunks.iter_mut().enumerate() {
                let most = chunk.info.count.min(MAX_CHUNK_POINTS);
                let min: [f64; 3] = std::array::from_fn(|k| chunk.info.min[k] - frame[k]);
                let size = chunk.info.size;
                let shown = cloud.visible
                    && slice.is_none_or(|s| s.overlaps(min, size))
                    && in_view(view, min, size);
                if !shown {
                    chunk.wanted = base.min(most);
                    chunk.draw = 0;
                    continue;
                }
                let d = distance_to_box(view.eye, min, size).max(1.0);
                let side = size[0].max(size[1]).max(0.1) / d * view.focal_px;
                let wanted = (side * side * DENSITY / (px * px)) as u64;
                chunk.wanted = wanted.clamp(base, u64::MAX).min(most);
                sum += chunk.wanted;
                seen.push((d, i, j));
            }
        }
        // Over budget: everyone draws the same share. Rounded down, so the sum holds.
        // What is held keeps its floor, for when the view turns.
        let share = (self.draw_budget as f64 / sum.max(1) as f64).min(1.0);
        for &(_, i, j) in &seen {
            let c = &mut self.clouds[i].chunks[j];
            let most = c.info.count.min(MAX_CHUNK_POINTS);
            let target = (c.wanted as f64 * share) as u64;
            c.wanted = target.clamp(base.min(most), most);
            c.draw = target.min(c.points.len() as u64);
        }
        // Nearest first.
        seen.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (_, i, j) in seen {
            self.request(i, j);
        }
        // Then the floor under everything else, so a turn of the view finds points.
        for i in 0..self.clouds.len() {
            for j in 0..self.clouds[i].chunks.len() {
                self.request(i, j);
            }
        }
        self.draws = self
            .clouds
            .iter()
            .flat_map(|c| {
                c.chunks.iter().enumerate().filter_map(|(j, chunk)| {
                    (chunk.draw > 0).then_some(DrawChunk {
                        id: c.id,
                        chunk: j,
                        count: chunk.draw as u32,
                    })
                })
            })
            .collect();
    }

    fn request(&mut self, i: usize, j: usize) {
        if self.in_flight >= MAX_IN_FLIGHT {
            return;
        }
        let c = &self.clouds[i].chunks[j];
        let have = c.points.len() as u64;
        if c.pending || c.failed || have >= c.wanted {
            return;
        }
        let most = c.info.count.min(MAX_CHUNK_POINTS);
        // A little ahead of need: a small move of the view should not reach the disk.
        let count = (c.wanted - have)
            .saturating_mul(2)
            .max(MIN_READ)
            .min(most - have);
        if !self.make_room(count, i, j) {
            return;
        }
        let id = self.clouds[i].id;
        self.clouds[i].chunks[j].pending = true;
        self.in_flight += 1;
        self.in_flight_points += count;
        self.send(Job::Read {
            id,
            chunk: j,
            start: have,
            count,
        });
    }

    /// Frees memory for `count` more points, from chunks holding the most beyond
    /// what they want. False when the budget has nothing left to give.
    fn make_room(&mut self, count: u64, keep_cloud: usize, keep_chunk: usize) -> bool {
        let held: u64 = self
            .clouds
            .iter()
            .flat_map(|c| &c.chunks)
            .map(|c| c.points.len() as u64)
            .sum();
        let mut over = (held + self.in_flight_points + count).saturating_sub(self.load_budget);
        if over == 0 {
            return true;
        }
        let mut surplus: Vec<(u64, usize, usize)> = Vec::new();
        for (i, cloud) in self.clouds.iter().enumerate() {
            for (j, c) in cloud.chunks.iter().enumerate() {
                let extra = (c.points.len() as u64).saturating_sub(c.wanted);
                if extra > 0 && (i, j) != (keep_cloud, keep_chunk) {
                    surplus.push((extra, i, j));
                }
            }
        }
        surplus.sort_unstable_by_key(|s| std::cmp::Reverse(s.0));
        for (extra, i, j) in surplus {
            if over == 0 {
                break;
            }
            let cut = extra.min(over);
            let c = &mut self.clouds[i].chunks[j];
            c.points.truncate(c.points.len() - cut as usize);
            c.points.shrink_to_fit();
            c.dirty = true;
            over -= cut;
        }
        over == 0
    }

    /// What this view draws, as [`CloudSet::plan`] decided.
    pub fn draws(&self) -> &[DrawChunk] {
        &self.draws
    }

    /// Hands out each chunk whose points changed since the last call.
    pub fn take_changed(&mut self, mut upload: impl FnMut(ChangedChunk<'_>)) {
        let frame = self.frame;
        for cloud in &mut self.clouds {
            for (j, c) in cloud.chunks.iter_mut().enumerate() {
                if std::mem::take(&mut c.dirty) {
                    upload(ChangedChunk {
                        id: cloud.id,
                        chunk: j,
                        min: std::array::from_fn(|k| c.info.min[k] - frame[k]),
                        size: c.info.size,
                        points: &c.points,
                    });
                }
            }
        }
    }

    /// The point under a project-local ray, among what is on screen: the nearest
    /// surface inside the cone, then the point closest to the ray on it.
    pub fn pick(&self, origin: [f64; 3], dir: [f64; 3], aperture: f64) -> Option<[f64; 3]> {
        if !self.snap || !aperture.is_finite() || aperture <= 0.0 {
            return None;
        }
        let tan = aperture.min(1.5).tan();
        let tan2 = tan * tan;
        let slice = self.active_slice();
        let mut hits: Vec<(f64, f64, [f64; 3])> = Vec::new();
        for cloud in self.clouds.iter().filter(|c| c.visible) {
            for c in &cloud.chunks {
                if c.draw == 0 {
                    continue;
                }
                let min: [f64; 3] = std::array::from_fn(|k| c.info.min[k] - self.frame[k]);
                if !cone_meets_box(origin, dir, tan, min, c.info.size) {
                    continue;
                }
                // Relative to the eye in f32, as cheap as a shader and just as precise.
                let rel: [f32; 3] = std::array::from_fn(|k| (min[k] - origin[k]) as f32);
                let scale: [f32; 3] = std::array::from_fn(|k| (c.info.size[k] / 65535.0) as f32);
                let d = dir.map(|v| v as f32);
                for p in &c.points[..c.draw as usize] {
                    let v: [f32; 3] =
                        std::array::from_fn(|k| rel[k] + f32::from(p.pos[k]) * scale[k]);
                    let along = v[0] * d[0] + v[1] * d[1] + v[2] * d[2];
                    if along <= 0.0 {
                        continue;
                    }
                    let len2 = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
                    let off2 = len2 - along * along;
                    if f64::from(off2) > f64::from(along * along) * tan2 {
                        continue;
                    }
                    let world = c.info.world(p);
                    let local: [f64; 3] = std::array::from_fn(|k| world[k] - self.frame[k]);
                    if slice.is_none_or(|s| s.contains(local)) {
                        hits.push((f64::from(along), f64::from(off2), local));
                    }
                }
            }
        }
        let nearest = hits.iter().map(|h| h.0).fold(f64::INFINITY, f64::min);
        hits.into_iter()
            .filter(|h| h.0 <= nearest * (1.0 + SAME_SURFACE))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|h| h.2)
    }
}

/// Whether a box, given from project zero, may show. Tests against the view's six
/// planes: false only when all corners lie outside one.
fn in_view(view: &View, min: [f64; 3], size: [f64; 3]) -> bool {
    let m = &view.view_proj;
    let corners = (0..8).map(|i: usize| -> [f64; 4] {
        let p: [f64; 3] = std::array::from_fn(|k| {
            min[k] + if i >> k & 1 == 1 { size[k] } else { 0.0 } - view.eye[k]
        });
        std::array::from_fn(|r| {
            (0..3).map(|c| f64::from(m[c][r]) * p[c]).sum::<f64>() + f64::from(m[3][r])
        })
    });
    let corners: Vec<[f64; 4]> = corners.collect();
    let outside = |f: fn(&[f64; 4]) -> f64| corners.iter().all(|c| f(c) < 0.0);
    !(outside(|c| c[3] + c[0])
        || outside(|c| c[3] - c[0])
        || outside(|c| c[3] + c[1])
        || outside(|c| c[3] - c[1])
        || outside(|c| c[2])
        || outside(|c| c[3] - c[2]))
}

fn distance_to_box(p: [f64; 3], min: [f64; 3], size: [f64; 3]) -> f64 {
    (0..3)
        .map(|k| {
            let d = (min[k] - p[k]).max(p[k] - min[k] - size[k]).max(0.0);
            d * d
        })
        .sum::<f64>()
        .sqrt()
}

/// Whether a cone around a ray may reach a box: the box grows by the cone's width at
/// the box's far side, then a slab test against the ray.
fn cone_meets_box(
    origin: [f64; 3],
    dir: [f64; 3],
    tan: f64,
    min: [f64; 3],
    size: [f64; 3],
) -> bool {
    let far = (0..3)
        .map(|k| {
            let d = (origin[k] - min[k])
                .abs()
                .max((origin[k] - min[k] - size[k]).abs());
            d * d
        })
        .sum::<f64>()
        .sqrt();
    let grow = far * tan;
    let (mut t0, mut t1) = (0.0f64, f64::INFINITY);
    for k in 0..3 {
        let (lo, hi) = (min[k] - grow, min[k] + size[k] + grow);
        if dir[k].abs() < 1e-12 {
            if origin[k] < lo || origin[k] > hi {
                return false;
            }
        } else {
            let (a, b) = ((lo - origin[k]) / dir[k], (hi - origin[k]) / dir[k]);
            t0 = t0.max(a.min(b));
            t1 = t1.min(a.max(b));
        }
    }
    t0 <= t1
}
