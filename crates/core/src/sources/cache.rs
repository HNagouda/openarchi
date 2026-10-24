//! Downloaded tiles kept on disk, so fetching a site again only downloads what
//! is new.
//!
//! The cache lives in the library folder rather than next to one project, so
//! neighbouring sites share their tiles too. It stays bounded: a tile older
//! than `MAX_AGE` counts as missing and is fetched again, because BGT and BAG
//! change, and `prune` deletes the oldest tiles once the cache outgrows
//! `MAX_BYTES`. A file's modified time is when its tile was downloaded.
//!
//! ```text
//! <cache>/tiles-v1/<dataset>/<column>_<row>.zst
//! ```

use std::cmp::Reverse;
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::sources::grid::Tile;
use crate::{Error, Result};

/// Data older than this is fetched again. Long enough that redrawing a site
/// over a few weeks of work stays fast, short enough that new buildings show up.
pub const MAX_AGE: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// A 1 km radius site leaves about 130 MB of tiles, two thirds of it
/// elevation, and a 300 m site about 25 MB. So this keeps about eight large
/// sites or forty small ones.
const MAX_BYTES: u64 = 1024 * 1024 * 1024;

/// Bumped when what a tile file holds changes. Older folders are deleted by `prune`.
const VERSION_FOLDER: &str = "tiles-v1";

const TILE_EXTENSION: &str = "zst";

/// Fast rather than small: tiles are written while the user waits.
const COMPRESSION_LEVEL: i32 = 3;

pub struct TileCache {
    dir: PathBuf,
}

impl TileCache {
    /// A cache in `dir`. Nothing is created until the first tile is written.
    pub fn at(dir: &Path) -> TileCache {
        TileCache {
            dir: dir.to_path_buf(),
        }
    }

    /// The tile's bytes, or `None` when it was never fetched or is older than `MAX_AGE`.
    pub fn read(&self, dataset: &str, tile: Tile) -> Result<Option<Vec<u8>>> {
        self.read_within(dataset, tile, MAX_AGE)
    }

    /// The tile's bytes, or `None` when it was never fetched or is older than
    /// `max_age`. For answers that go out of date sooner than map data does.
    pub fn read_within(
        &self,
        dataset: &str,
        tile: Tile,
        max_age: Duration,
    ) -> Result<Option<Vec<u8>>> {
        let path = self.tile_path(dataset, tile);
        let metadata = match fs::metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(Error::io(&path, error)),
        };
        let modified = metadata.modified().map_err(|e| Error::io(&path, e))?;
        if is_older_than(modified, max_age) {
            return Ok(None);
        }

        let compressed = fs::read(&path).map_err(|e| Error::io(&path, e))?;
        let bytes = zstd::decode_all(compressed.as_slice()).map_err(|e| Error::io(&path, e))?;
        Ok(Some(bytes))
    }

    /// Stores a tile, replacing any older copy.
    pub fn write(&self, dataset: &str, tile: Tile, bytes: &[u8]) -> Result<()> {
        let path = self.tile_path(dataset, tile);
        let folder = self.dir.join(VERSION_FOLDER).join(dataset);
        fs::create_dir_all(&folder).map_err(|e| Error::io(&folder, e))?;
        let compressed =
            zstd::encode_all(bytes, COMPRESSION_LEVEL).map_err(|e| Error::io(&path, e))?;

        // The app and the MCP server may write the same tile at once, so each
        // write gets its own temporary name before the rename.
        let temp = path.with_extension(format!("{:08x}.tmp", fastrand::u32(..)));
        let mut file = fs::File::create(&temp).map_err(|e| Error::io(&temp, e))?;
        file.write_all(&compressed)
            .map_err(|e| Error::io(&temp, e))?;
        // Windows cannot rename a file that is still open.
        drop(file);
        fs::rename(&temp, &path).map_err(|e| Error::io(&path, e))
    }

    /// Deletes stale tiles, folders of older cache versions, and then the
    /// oldest tiles until the cache fits in `MAX_BYTES`.
    pub fn prune(&self) -> Result<()> {
        if !self.dir.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(&self.dir).map_err(|e| Error::io(&self.dir, e))? {
            let path = entry.map_err(|e| Error::io(&self.dir, e))?.path();
            if path.file_name().is_some_and(|name| name != VERSION_FOLDER) {
                remove(&path)?;
            }
        }

        let mut kept = Vec::new();
        for (path, modified, size) in self.tile_files()? {
            if is_older_than(modified, MAX_AGE) {
                fs::remove_file(&path).map_err(|e| Error::io(&path, e))?;
            } else {
                kept.push((path, modified, size));
            }
        }

        // Newest first, so the tiles past the budget are the oldest ones.
        kept.sort_by_key(|entry| Reverse(entry.1));
        let mut total = 0;
        for (path, _, size) in kept {
            total += size;
            if total > MAX_BYTES {
                fs::remove_file(&path).map_err(|e| Error::io(&path, e))?;
            }
        }
        Ok(())
    }

    /// Every tile file with its download time and size on disk.
    fn tile_files(&self) -> Result<Vec<(PathBuf, SystemTime, u64)>> {
        let root = self.dir.join(VERSION_FOLDER);
        if !root.exists() {
            return Ok(Vec::new());
        }

        let mut files = Vec::new();
        for dataset in fs::read_dir(&root).map_err(|e| Error::io(&root, e))? {
            let folder = dataset.map_err(|e| Error::io(&root, e))?.path();
            for entry in fs::read_dir(&folder).map_err(|e| Error::io(&folder, e))? {
                let path = entry.map_err(|e| Error::io(&folder, e))?.path();
                let metadata = fs::metadata(&path).map_err(|e| Error::io(&path, e))?;
                let modified = metadata.modified().map_err(|e| Error::io(&path, e))?;
                files.push((path, modified, metadata.len()));
            }
        }
        Ok(files)
    }

    fn tile_path(&self, dataset: &str, tile: Tile) -> PathBuf {
        self.dir
            .join(VERSION_FOLDER)
            .join(dataset)
            .join(format!("{}_{}.{TILE_EXTENSION}", tile.column, tile.row))
    }
}

/// A clock set backwards makes a tile look new; that only delays a refetch.
fn is_older_than(modified: SystemTime, max_age: Duration) -> bool {
    SystemTime::now()
        .duration_since(modified)
        .is_ok_and(|age| age > max_age)
}

fn remove(path: &Path) -> Result<()> {
    let result = if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    };
    result.map_err(|e| Error::io(path, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const DATASET: &str = "bag-pand";
    const TILE: Tile = Tile {
        column: 482,
        row: 1950,
    };

    fn age(path: &Path, by: Duration) {
        let file = fs::File::options().write(true).open(path).unwrap();
        file.set_modified(SystemTime::now() - by).unwrap();
    }

    #[test]
    fn a_written_tile_reads_back_the_same_bytes() {
        let folder = TempDir::new().unwrap();
        let cache = TileCache::at(folder.path());
        cache.write(DATASET, TILE, b"[1, 2, 3]").unwrap();
        assert_eq!(cache.read(DATASET, TILE).unwrap().unwrap(), b"[1, 2, 3]");
    }

    #[test]
    fn a_tile_never_fetched_is_missing() {
        let folder = TempDir::new().unwrap();
        let cache = TileCache::at(folder.path());
        assert_eq!(cache.read(DATASET, TILE).unwrap(), None);
    }

    #[test]
    fn a_tile_older_than_the_maximum_age_is_missing_and_pruned() {
        let folder = TempDir::new().unwrap();
        let cache = TileCache::at(folder.path());
        cache.write(DATASET, TILE, b"old").unwrap();
        let path = cache.tile_path(DATASET, TILE);
        age(&path, MAX_AGE + Duration::from_secs(60));

        assert_eq!(cache.read(DATASET, TILE).unwrap(), None);
        cache.prune().unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn prune_removes_folders_of_older_cache_versions() {
        let folder = TempDir::new().unwrap();
        let cache = TileCache::at(folder.path());
        let old_version = folder.path().join("tiles-v0/bag-pand");
        fs::create_dir_all(&old_version).unwrap();
        cache.write(DATASET, TILE, b"new").unwrap();

        cache.prune().unwrap();

        assert!(!folder.path().join("tiles-v0").exists());
        assert!(cache.read(DATASET, TILE).unwrap().is_some());
    }
}
