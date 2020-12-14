// Copyright 2020  Israel Basurto
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use chrono;
use globset::{Glob, GlobSetBuilder};
use seahash::SeaHasher;
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::hash::Hasher;
use std::io;
use std::path;
use unicode_normalization::UnicodeNormalization;

const MEBI: usize = 1 << 20;

#[derive(Debug)]
pub struct Options {
    pub base_dir: path::PathBuf,
    pub exclude_set: HashSet<String>,
    exclude_globset: globset::GlobSet,
}
impl Options {
    pub fn new<T>(base_dir: path::PathBuf, globs_args: T) -> Result<Options, globset::Error>
    where
        T: IntoIterator,
        T::Item: ToString,
    {
        let mut exclude_set = HashSet::new();
        let mut globset_builder = GlobSetBuilder::new();
        for glob in globs_args.into_iter() {
            let glob = glob.to_string();
            if exclude_set.insert(glob.clone()) {
                globset_builder.add(Glob::new(glob.as_str())?);
            }
        }
        let exclude_globset = globset_builder.build()?;
        Ok(Options {
            base_dir,
            exclude_set,
            exclude_globset,
        })
    }
}

#[derive(Debug, Default)]
pub struct Summary {
    pub found_dirs: usize,
    pub found_symlinks: usize,
    pub found_files: usize,
    pub files_total_size: u64,
}

#[derive(Debug)]
struct ConcatHash(Vec<u8>);

#[derive(Serialize, Debug)]
pub struct SymlinkMetaData {
    target: path::PathBuf,
}

#[derive(Serialize, Debug)]
pub enum DirEntry {
    Dir,
    Symlink {
        target: path::PathBuf,
    },
    File {
    len: u64,
    #[serde(serialize_with = "serialize_date_time")]
    mtime: chrono::DateTime<chrono::offset::Utc>,
    #[serde(serialize_with = "serialize_concat_hash")]
    hash: ConcatHash,
    },
}

#[derive(Serialize, Debug, Default)]
pub struct DirTree(BTreeMap<path::PathBuf, DirEntry>);

fn log_progress(summary: &Summary, hashed_bytes: Option<(u64, u64)>) {
    eprint!(
        "\r{:>6} dirs, {:>6} symlinks, {:>6} files found",
        summary.found_dirs, summary.found_symlinks, summary.found_files
    );
    if let Some(hashed_bytes) = hashed_bytes {
        eprint!(
            //234567890123       12       1       12345
            "; hashing... {:>5.1}% {:>8.3}/{:>8.3} MiB",
            100.0 * hashed_bytes.0 as f64 / hashed_bytes.1 as f64,
            hashed_bytes.0 as f64 / (1024.0 * 1024.0),
            hashed_bytes.1 as f64 / (1024.0 * 1024.0),
        )
    } else {
        //                    12345  12345678 12345678
        //       1234567890123     12        1        12345
        eprint!("                                          ")
    }
}

pub fn visit_dir_tree(
    options: &Options,
    summary: &mut Summary,
    dir_tree: &mut DirTree,
    dir: impl AsRef<path::Path>,
    prefix: &impl AsRef<path::Path>,
) -> io::Result<()> {
    for dir_entry in fs::read_dir(dir)? {
        let dir_entry = dir_entry?;
        let file_type = dir_entry.file_type()?;
        let path_without_prefix = dir_entry
            .path()
            .strip_prefix(prefix)
            .unwrap()
            .to_str()
            .unwrap()
            .nfc()
            .collect::<String>()
            .into();
        if options.exclude_globset.is_match(&path_without_prefix) {
            // ignore excluded paths
        } else if file_type.is_dir() {
            dir_tree.0.insert(path_without_prefix, DirEntry::Dir);
            summary.found_dirs += 1;
            log_progress(summary, None);
            visit_dir_tree(options, summary, dir_tree, dir_entry.path(), prefix)?
        } else if file_type.is_symlink() {
            let target = fs::read_link(dir_entry.path())?;
            dir_tree
                .0
                .insert(path_without_prefix, DirEntry::Symlink { target });
            summary.found_symlinks += 1;
            log_progress(summary, None);
        } else if file_type.is_file() {
            let md = dir_entry.metadata()?;
            let mut total_hashed = 0_u64;
            dir_tree.0.insert(
                path_without_prefix,
                DirEntry::File {
                    len: md.len(),
                    mtime: md.modified()?.into(),
                    hash: concat_hash(dir_entry.path(), |len| {
                        total_hashed += len;
                        log_progress(summary, Some((total_hashed, md.len())));
                    })?,
                },
            );
            summary.found_files += 1;
            summary.files_total_size += md.len();
            log_progress(summary, None);
        }
    }
    Ok(())
}

// custom serialization for DateTime
fn serialize_date_time<S>(
    dt: &chrono::DateTime<chrono::offset::Utc>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    // display milli/nanoseconds if they are non-zero
    serializer.serialize_str(&dt.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true))
}

fn bufcopy<F: FnMut(u64)>(
    buf: &mut [u8],
    reader: &mut impl io::Read,
    writer: &mut impl io::Write,
    mut log: F,
) -> io::Result<()> {
    loop {
        let len = match reader.read(buf) {
            Ok(0) => return Ok(()),
            Ok(len) => len,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        writer.write_all(&buf[..len])?;
        log(len as u64);
    }
}

struct ConcatHasherToWriteAdapter<H: Hasher> {
    hasher: H,
    concat_hash: ConcatHash,
}
impl<H: Hasher> io::Write for ConcatHasherToWriteAdapter<H> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Hasher::write(&mut self.hasher, buf);
        self.concat_hash
            .0
            .extend(Vec::from(self.hasher.finish().to_le_bytes()));
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn concat_hash<F: FnMut(u64)>(path: impl AsRef<path::Path>, log: F) -> io::Result<ConcatHash> {
    let mut file = fs::File::open(path)?;
    let hasher = SeaHasher::new();
    let mut buf = [0; 1 * MEBI];
    let mut concat_adapter = ConcatHasherToWriteAdapter {
        hasher,
        concat_hash: ConcatHash(Vec::new()),
    };
    bufcopy(&mut buf, &mut file, &mut concat_adapter, log)?;
    Ok(concat_adapter.concat_hash)
}

fn serialize_concat_hash<S>(concat_hash: &ConcatHash, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(
        &concat_hash
            .0
            .iter()
            .map(|x| format!("{:X?}", x))
            .collect::<String>(),
    )
}
