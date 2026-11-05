//! LAS point clouds cached as shuffled spatial chunks. No GPU code here.

use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File, OpenOptions};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};

use bytemuck::{Pod, Zeroable};
use las::{PointDataBuilder, Reader};
use serde::{Deserialize, Serialize};

const MAGIC: &[u8; 8] = b"WPCLOUD1";
const HEADER_SIZE: u64 = 16;
const POINT_SIZE: u64 = std::mem::size_of::<CloudPoint>() as u64;
const BATCH: u64 = 65_536;
const TARGET_CHUNK: u64 = 100_000;
// Vec growth can double capacity, so flush data at half the allocation ceiling.
const BUFFER_FLUSH: usize = 32 * 1024 * 1024;

/// One point as it sits on disk and in a vertex buffer.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct CloudPoint {
    /// x, y, z from the chunk's `min` across its `size`, 0..=65535. The fourth is
    /// zero: vertex formats have no three-part u16.
    pub pos: [u16; 4],
    pub color: [u8; 4],
}

/// One cell of the grid. World coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ChunkInfo {
    /// True bounds of the points in it.
    pub min: [f64; 3],
    pub size: [f64; 3],
    pub count: u64,
    /// Where its points start in the file, in bytes.
    pub offset: u64,
}

impl ChunkInfo {
    /// A point's world position.
    pub fn world(&self, p: &CloudPoint) -> [f64; 3] {
        std::array::from_fn(|k| self.min[k] + self.size[k] * f64::from(p.pos[k]) / 65535.0)
    }
}

/// What a cache holds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CloudInfo {
    /// `None`: the LAS named no CRS, or none this code can read as EPSG.
    pub epsg: Option<u32>,
    pub points: u64,
    pub min: [f64; 3],
    pub max: [f64; 3],
    pub chunks: Vec<ChunkInfo>,
}

#[derive(Debug)]
pub enum ImportError {
    /// Not a LAS file this reader understands, or a damaged one.
    Las(String),
    NoPoints,
    /// Fewer points than the header promises.
    Truncated {
        expected: u64,
        got: u64,
    },
    /// Coordinates that are not finite numbers.
    BadCoordinates,
    Cancelled,
    Io(std::io::Error),
    /// The cache file is not one this code wrote, or is damaged.
    BadCache(String),
    /// The reader panicked on this file.
    Panicked,
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Las(e) => write!(f, "Berkas LAS tidak dapat dibaca atau rusak: {e}"),
            Self::NoPoints => write!(f, "Berkas LAS tidak berisi titik."),
            Self::Truncated { expected, got } => write!(
                f,
                "Data titik LAS terpotong: {got} dari {expected} titik. Salin ulang berkas sumber."
            ),
            Self::BadCoordinates => write!(f, "Berkas LAS memuat koordinat yang tidak valid."),
            Self::Cancelled => write!(f, "Impor point cloud dibatalkan."),
            Self::Io(e) => write!(f, "Berkas point cloud tidak dapat dibaca atau ditulis: {e}"),
            Self::BadCache(_) => write!(f, "Cache point cloud rusak; impor ulang berkas LAS."),
            Self::Panicked => write!(
                f,
                "Pembaca LAS berhenti secara tidak terduga; periksa berkas sumber."
            ),
        }
    }
}

impl std::error::Error for ImportError {}
impl From<std::io::Error> for ImportError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

/// Which pass an import is in.
pub const PHASE_SCAN: u8 = 0;
pub const PHASE_WRITE: u8 = 1;
pub const PHASE_SHUFFLE: u8 = 2;

/// Shared with the thread running an import.
#[derive(Debug, Default)]
pub struct Progress {
    pub phase: AtomicU8,
    /// Of `total`, in points for all phases.
    pub done: AtomicU64,
    pub total: AtomicU64,
    pub cancel: AtomicBool,
}

/// Where the cache of `source` belongs in `cache_dir`.
pub fn cache_path(source: &Path, cache_dir: &Path) -> std::io::Result<PathBuf> {
    let canonical = source.canonicalize()?;
    let metadata = canonical.metadata()?;
    let mut hasher = DefaultHasher::new();
    canonical.hash(&mut hasher);
    metadata.len().hash(&mut hasher);
    metadata.modified()?.hash(&mut hasher);
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("cloud");
    Ok(cache_dir.join(format!("{stem}-{:016x}.wpc", hasher.finish())))
}

#[derive(Clone, Copy)]
struct CellStats {
    count: u64,
    min: [f64; 3],
    max: [f64; 3],
}
struct Scan {
    expected: u64,
    points: u64,
    side: u32,
    header_min: [f64; 2],
    header_max: [f64; 2],
    min: [f64; 3],
    max: [f64; 3],
    color_max: u16,
    has_color: bool,
    epsg: Option<u32>,
    cells: BTreeMap<u32, CellStats>,
}

fn set_progress(p: &Progress, phase: u8, total: u64) {
    p.phase.store(phase, Ordering::Relaxed);
    p.total.store(total, Ordering::Relaxed);
    p.done.store(0, Ordering::Relaxed);
}
fn cancelled(p: &Progress) -> Result<(), ImportError> {
    if p.cancel.load(Ordering::Relaxed) {
        Err(ImportError::Cancelled)
    } else {
        Ok(())
    }
}
fn las_error(e: las::Error) -> ImportError {
    ImportError::Las(e.to_string())
}

fn cell_index(x: f64, y: f64, scan: &Scan) -> u32 {
    fn axis(v: f64, min: f64, max: f64, side: u32) -> u32 {
        if max <= min {
            return 0;
        }
        (((v - min) / (max - min) * f64::from(side)).floor())
            .max(0.0)
            .min(f64::from(side - 1)) as u32
    }
    axis(x, scan.header_min[0], scan.header_max[0], scan.side)
        + axis(y, scan.header_min[1], scan.header_max[1], scan.side) * scan.side
}

fn epsg(reader: &Reader) -> Option<u32> {
    if let Ok(Some(crs)) = reader.header().get_geotiff_crs()
        && let Some(value) = crs.get_projected_crs_geo_key_value()
        && value != 0
        && value != 32767
    {
        return Some(u32::from(value));
    }
    let text = std::str::from_utf8(reader.header().get_wkt_crs_bytes()?).ok()?;
    parse_wkt_epsg(text)
}
fn parse_wkt_epsg(text: &str) -> Option<u32> {
    let upper = text.to_ascii_uppercase();
    let mut hits = Vec::new();
    for marker in ["AUTHORITY[\"EPSG\",\"", "ID[\"EPSG\","] {
        let mut base = 0;
        while let Some(at) = upper[base..].find(marker) {
            let start = base + at + marker.len();
            let tail = upper[start..].trim_start().trim_start_matches('"');
            let end = tail
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(tail.len());
            if end > 0 {
                hits.push((start, tail[..end].parse::<u32>().ok()?));
            }
            base = start;
        }
    }
    hits.into_iter().max_by_key(|hit| hit.0).map(|hit| hit.1)
}

fn scan_source(source: &Path, p: &Progress) -> Result<Scan, ImportError> {
    cancelled(p)?;
    let mut reader = Reader::from_path(source).map_err(las_error)?;
    let expected = reader.header().number_of_points();
    set_progress(p, PHASE_SCAN, expected);
    let bounds = reader.header().bounds();
    let side = ((expected.div_ceil(TARGET_CHUNK) as f64).sqrt().ceil() as u32).clamp(1, 4096);
    let mut scan = Scan {
        expected,
        points: 0,
        side,
        header_min: [bounds.min.x, bounds.min.y],
        header_max: [bounds.max.x, bounds.max.y],
        min: [f64::INFINITY; 3],
        max: [f64::NEG_INFINITY; 3],
        color_max: 0,
        has_color: reader.header().point_format().has_color,
        epsg: epsg(&reader),
        cells: BTreeMap::new(),
    };
    let mut data = PointDataBuilder::new().for_header(reader.header()).build();
    loop {
        cancelled(p)?;
        let count = match reader.fill_points(BATCH, &mut data) {
            Ok(n) => n,
            Err(_) if scan.points < expected => {
                return Err(ImportError::Truncated {
                    expected,
                    got: scan.points,
                });
            }
            Err(e) => return Err(las_error(e)),
        };
        if count == 0 {
            break;
        }
        let colors: Vec<_> = data.rgb().map(|it| it.collect()).unwrap_or_default();
        for (i, ((x, y), z)) in data.x().zip(data.y()).zip(data.z()).enumerate() {
            if !x.is_finite() || !y.is_finite() || !z.is_finite() {
                return Err(ImportError::BadCoordinates);
            }
            let xyz = [x, y, z];
            for (k, &value) in xyz.iter().enumerate() {
                scan.min[k] = scan.min[k].min(value);
                scan.max[k] = scan.max[k].max(value);
            }
            let index = cell_index(x, y, &scan);
            let cell = scan.cells.entry(index).or_insert(CellStats {
                count: 0,
                min: [f64::INFINITY; 3],
                max: [f64::NEG_INFINITY; 3],
            });
            cell.count += 1;
            for (k, &value) in xyz.iter().enumerate() {
                cell.min[k] = cell.min[k].min(value);
                cell.max[k] = cell.max[k].max(value);
            }
            if let Some(&(r, g, b)) = colors.get(i) {
                scan.color_max = scan.color_max.max(r).max(g).max(b);
            }
        }
        scan.points += count;
        p.done.store(scan.points, Ordering::Relaxed);
    }
    if scan.points == 0 {
        return Err(ImportError::NoPoints);
    }
    if scan.points < expected {
        return Err(ImportError::Truncated {
            expected,
            got: scan.points,
        });
    }
    Ok(scan)
}

fn quantize(v: f64, min: f64, size: f64) -> u16 {
    if size == 0.0 {
        0
    } else {
        ((v - min) / size * 65535.0).round().clamp(0.0, 65535.0) as u16
    }
}
fn elevation_color(z: f64, min: f64, max: f64) -> [u8; 4] {
    let t = if max == min {
        0.5
    } else {
        ((z - min) / (max - min)).clamp(0.0, 1.0)
    };
    let (r, g, b) = if t <= 0.5 {
        (0.0, t * 2.0, 1.0 - t * 2.0)
    } else {
        ((t - 0.5) * 2.0, (1.0 - t) * 2.0, 0.0)
    };
    [
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
        255,
    ]
}

fn flush(
    file: &mut File,
    buffers: &mut [Vec<CloudPoint>],
    cursors: &mut [u64],
) -> Result<(), ImportError> {
    for (buffer, cursor) in buffers.iter_mut().zip(cursors) {
        if buffer.is_empty() {
            continue;
        }
        file.seek(SeekFrom::Start(*cursor))?;
        file.write_all(bytemuck::cast_slice(buffer))?;
        *cursor += buffer.len() as u64 * POINT_SIZE;
        *buffer = Vec::new();
    }
    Ok(())
}

fn write_points(
    source: &Path,
    file: &mut File,
    scan: &Scan,
    chunks: &[ChunkInfo],
    p: &Progress,
) -> Result<(), ImportError> {
    set_progress(p, PHASE_WRITE, scan.points);
    let mut reader = Reader::from_path(source).map_err(las_error)?;
    let mut data = PointDataBuilder::new().for_header(reader.header()).build();
    let lookup: HashMap<u32, usize> = scan
        .cells
        .keys()
        .enumerate()
        .map(|(i, &key)| (key, i))
        .collect();
    let mut buffers = vec![Vec::new(); chunks.len()];
    let mut cursors: Vec<_> = chunks.iter().map(|chunk| chunk.offset).collect();
    let mut buffered = 0;
    let mut done = 0;
    loop {
        cancelled(p)?;
        let count = reader.fill_points(BATCH, &mut data).map_err(las_error)?;
        if count == 0 {
            break;
        }
        let colors: Vec<_> = data.rgb().map(|it| it.collect()).unwrap_or_default();
        for (i, ((x, y), z)) in data.x().zip(data.y()).zip(data.z()).enumerate() {
            let ci = lookup[&cell_index(x, y, scan)];
            let chunk = chunks[ci];
            let color = if scan.has_color && scan.color_max > 0 {
                let (r, g, b) = colors[i];
                let cvt = |v: u16| {
                    if scan.color_max > 255 {
                        (v >> 8) as u8
                    } else {
                        v as u8
                    }
                };
                [cvt(r), cvt(g), cvt(b), 255]
            } else {
                elevation_color(z, scan.min[2], scan.max[2])
            };
            buffers[ci].push(CloudPoint {
                pos: [
                    quantize(x, chunk.min[0], chunk.size[0]),
                    quantize(y, chunk.min[1], chunk.size[1]),
                    quantize(z, chunk.min[2], chunk.size[2]),
                    0,
                ],
                color,
            });
            buffered += std::mem::size_of::<CloudPoint>();
            if buffered >= BUFFER_FLUSH {
                flush(file, &mut buffers, &mut cursors)?;
                buffered = 0;
            }
        }
        done += count;
        p.done.store(done, Ordering::Relaxed);
    }
    flush(file, &mut buffers, &mut cursors)?;
    if done < scan.expected {
        return Err(ImportError::Truncated {
            expected: scan.expected,
            got: done,
        });
    }
    Ok(())
}

struct SplitMix64(u64);
impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
}
fn shuffle(
    file: &mut File,
    chunks: &[ChunkInfo],
    points: u64,
    p: &Progress,
) -> Result<(), ImportError> {
    set_progress(p, PHASE_SHUFFLE, points);
    let mut done = 0;
    for (index, chunk) in chunks.iter().enumerate() {
        cancelled(p)?;
        let count = usize::try_from(chunk.count).map_err(|_| bad_cache("Chunk terlalu besar."))?;
        let mut values = vec![CloudPoint::zeroed(); count];
        file.seek(SeekFrom::Start(chunk.offset))?;
        file.read_exact(bytemuck::cast_slice_mut(&mut values))?;
        let mut rng = SplitMix64(index as u64);
        for i in (1..values.len()).rev() {
            if i % BATCH as usize == 0 {
                cancelled(p)?;
            }
            let j = (rng.next() % (i as u64 + 1)) as usize;
            values.swap(i, j);
        }
        file.seek(SeekFrom::Start(chunk.offset))?;
        file.write_all(bytemuck::cast_slice(&values))?;
        done += chunk.count;
        p.done.store(done, Ordering::Relaxed);
    }
    Ok(())
}

fn import_inner(
    source: &Path,
    cache: &Path,
    temp: &Path,
    p: &Progress,
    temp_created: &AtomicBool,
) -> Result<(), ImportError> {
    if let Some(parent) = cache.parent().filter(|path| !path.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    let scan = scan_source(source, p)?;
    cancelled(p)?;
    let mut offset = HEADER_SIZE;
    let mut chunks = Vec::with_capacity(scan.cells.len());
    for cell in scan.cells.values() {
        chunks.push(ChunkInfo {
            min: cell.min,
            size: std::array::from_fn(|k| cell.max[k] - cell.min[k]),
            count: cell.count,
            offset,
        });
        offset = offset
            .checked_add(
                cell.count
                    .checked_mul(POINT_SIZE)
                    .ok_or_else(|| bad_cache("Ukuran titik meluap."))?,
            )
            .ok_or_else(|| bad_cache("Ukuran cache meluap."))?;
    }
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(temp)?;
    temp_created.store(true, Ordering::Relaxed);
    file.write_all(MAGIC)?;
    file.write_all(&0_u64.to_le_bytes())?;
    file.set_len(offset)?;
    write_points(source, &mut file, &scan, &chunks, p)?;
    shuffle(&mut file, &chunks, scan.points, p)?;
    let info = CloudInfo {
        epsg: scan.epsg,
        points: scan.points,
        min: scan.min,
        max: scan.max,
        chunks,
    };
    let table = serde_json::to_vec(&info).map_err(|e| bad_cache(e.to_string()))?;
    file.seek(SeekFrom::Start(offset))?;
    file.write_all(&table)?;
    file.seek(SeekFrom::Start(8))?;
    file.write_all(&offset.to_le_bytes())?;
    file.sync_all()?;
    drop(file);
    fs::rename(temp, cache)?;
    Ok(())
}

/// Builds the cache at `cache` from the LAS at `source`.
pub fn import_las(source: &Path, cache: &Path, progress: &Progress) -> Result<(), ImportError> {
    let name = cache
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or("cache.wpc");
    let temp = cache.with_file_name(format!("{name}.{}.tmp", std::process::id()));
    let temp_created = AtomicBool::new(false);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        import_inner(source, cache, &temp, progress, &temp_created)
    }));
    match result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => {
            if temp_created.load(Ordering::Relaxed) {
                let _ = fs::remove_file(&temp);
            }
            Err(e)
        }
        Err(_) => {
            if temp_created.load(Ordering::Relaxed) {
                let _ = fs::remove_file(&temp);
            }
            Err(ImportError::Panicked)
        }
    }
}

/// An open cache.
#[derive(Debug)]
pub struct CloudFile {
    pub info: CloudInfo,
    file: File,
}
fn bad_cache(message: impl Into<String>) -> ImportError {
    ImportError::BadCache(message.into())
}

impl CloudFile {
    /// Checks the file before trusting any of it.
    pub fn open(cache: &Path) -> Result<Self, ImportError> {
        let mut file = File::open(cache)?;
        let length = file.metadata()?.len();
        if length < HEADER_SIZE {
            return Err(bad_cache("Header cache tidak lengkap."));
        }
        let mut header = [0_u8; 16];
        file.read_exact(&mut header)?;
        if &header[..8] != MAGIC {
            return Err(bad_cache("Magic cache salah."));
        }
        let table_offset = u64::from_le_bytes(
            header[8..]
                .try_into()
                .map_err(|_| bad_cache("Header cache salah."))?,
        );
        if !(HEADER_SIZE..=length).contains(&table_offset) {
            return Err(bad_cache("Offset tabel di luar berkas."));
        }
        let table_len = usize::try_from(length - table_offset)
            .map_err(|_| bad_cache("Tabel terlalu besar."))?;
        let max_table = (table_offset - HEADER_SIZE)
            .checked_mul(32)
            .and_then(|size| size.checked_add(4096))
            .ok_or_else(|| bad_cache("Tabel terlalu besar."))?;
        if table_len as u64 > max_table {
            return Err(bad_cache("Tabel terlalu besar."));
        }
        let mut table = Vec::new();
        table
            .try_reserve_exact(table_len)
            .map_err(|_| bad_cache("Tabel terlalu besar."))?;
        table.resize(table_len, 0);
        file.seek(SeekFrom::Start(table_offset))?;
        file.read_exact(&mut table)?;
        let info: CloudInfo =
            serde_json::from_slice(&table).map_err(|e| bad_cache(e.to_string()))?;
        if info.min.iter().chain(&info.max).any(|v| !v.is_finite()) {
            return Err(bad_cache("Batas cloud tidak finite."));
        }
        if info.points == 0 || info.chunks.is_empty() || (0..3).any(|k| info.min[k] > info.max[k]) {
            return Err(bad_cache("Batas atau jumlah cloud tidak valid."));
        }
        let expected = HEADER_SIZE
            .checked_add(
                info.points
                    .checked_mul(POINT_SIZE)
                    .ok_or_else(|| bad_cache("Jumlah titik meluap."))?,
            )
            .ok_or_else(|| bad_cache("Ukuran titik meluap."))?;
        if expected != table_offset {
            return Err(bad_cache("Offset tabel tidak cocok dengan jumlah titik."));
        }
        let mut ranges = Vec::new();
        ranges
            .try_reserve_exact(info.chunks.len())
            .map_err(|_| bad_cache("Jumlah chunk terlalu besar."))?;
        let mut sum = 0_u64;
        for chunk in &info.chunks {
            if chunk.count == 0 {
                return Err(bad_cache("Chunk kosong tidak valid."));
            }
            if chunk.min.iter().any(|v| !v.is_finite())
                || chunk.size.iter().any(|v| !v.is_finite() || *v < 0.0)
            {
                return Err(bad_cache("Batas chunk tidak valid."));
            }
            let end = chunk
                .offset
                .checked_add(
                    chunk
                        .count
                        .checked_mul(POINT_SIZE)
                        .ok_or_else(|| bad_cache("Ukuran chunk meluap."))?,
                )
                .ok_or_else(|| bad_cache("Offset chunk meluap."))?;
            if chunk.offset < HEADER_SIZE || end > table_offset {
                return Err(bad_cache("Chunk di luar area titik."));
            }
            ranges.push((chunk.offset, end));
            sum = sum
                .checked_add(chunk.count)
                .ok_or_else(|| bad_cache("Jumlah titik meluap."))?;
        }
        ranges.sort_unstable();
        if ranges.windows(2).any(|pair| pair[0].1 > pair[1].0) {
            return Err(bad_cache("Chunk saling tumpang tindih."));
        }
        if sum != info.points {
            return Err(bad_cache("Jumlah titik chunk tidak cocok."));
        }
        Ok(Self { info, file })
    }

    /// Points `start..start + count` of a chunk, cut off at its end.
    pub fn read(
        &mut self,
        chunk: usize,
        start: u64,
        count: u64,
    ) -> Result<Vec<CloudPoint>, ImportError> {
        let info = self
            .info
            .chunks
            .get(chunk)
            .ok_or_else(|| bad_cache("Indeks chunk tidak dikenal."))?;
        if start >= info.count || count == 0 {
            return Ok(Vec::new());
        }
        let count = count.min(info.count - start);
        let count =
            usize::try_from(count).map_err(|_| bad_cache("Permintaan baca terlalu besar."))?;
        let mut points = Vec::new();
        points
            .try_reserve_exact(count)
            .map_err(|_| bad_cache("Permintaan baca terlalu besar."))?;
        points.resize(count, CloudPoint::zeroed());
        let offset = info
            .offset
            .checked_add(
                start
                    .checked_mul(POINT_SIZE)
                    .ok_or_else(|| bad_cache("Offset baca meluap."))?,
            )
            .ok_or_else(|| bad_cache("Offset baca meluap."))?;
        self.file.seek(SeekFrom::Start(offset))?;
        self.file
            .read_exact(bytemuck::cast_slice_mut(&mut points))?;
        Ok(points)
    }
}
