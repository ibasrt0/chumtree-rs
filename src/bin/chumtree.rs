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
use globset::{Glob, GlobSet, GlobSetBuilder};
use seahash::SeaHasher;
use serde::Serialize;
use std::cmp::{PartialEq, PartialOrd};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::hash::Hasher;
use std::io;
use std::path;
use unicode_normalization::UnicodeNormalization;

const MEBI: usize = 1 << 20;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ConcatHash(Vec<u8>);

#[derive(Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct FileMetaData {
    path: path::PathBuf,
    len: u64,
    #[serde(serialize_with = "serialize_date_time")]
    modified: chrono::DateTime<chrono::offset::Utc>,
    #[serde(serialize_with = "serialize_concat_hash")]
    hash: ConcatHash,
}

#[derive(Serialize, Debug)]
struct DirTree {
    #[serde(serialize_with = "serialize_date_time")]
    timestamp: chrono::DateTime<chrono::offset::Utc>,

    base_dir: path::PathBuf,
    exclude_set: HashSet<String>,
    #[serde(skip)]
    exclude_globset: globset::GlobSet,

    found_dirs: usize,
    found_symlinks: usize,
    found_files: usize,
    files_total_size: u64,

    dirs: Vec<path::PathBuf>,
    symlinks: Vec<(path::PathBuf, path::PathBuf)>,
    files: Vec<FileMetaData>,
}

impl DirTree {
    fn new(base_dir: path::PathBuf, exclude_set: HashSet<String>) -> DirTree {
        let exclude_gobset = exclude_set
            .iter()
            .fold(&mut GlobSetBuilder::new(), |builder, glob| {
                if let Ok(glob) = Glob::new(glob) {
                    builder.add(glob);
                }
                builder
            })
            .build()
            .unwrap_or_else(|_| GlobSet::empty());

        DirTree {
            timestamp: chrono::offset::Utc::now(),

            base_dir: base_dir,
            exclude_set: exclude_set,
            exclude_globset: exclude_gobset,

            found_dirs: 0,
            found_symlinks: 0,
            found_files: 0,
            files_total_size: 0,

            dirs: Vec::new(),
            symlinks: Vec::new(),
            files: Vec::new(),
        }
    }

    fn log_progress(&self, hashed_bytes: Option<(u64, u64)>) {
        eprint!(
            "\r{:>6} dirs, {:>6} symlinks, {:>6} files found",
            self.dirs.len(),
            self.symlinks.len(),
            self.files.len()
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
    fn visit_dir_tree(
        &mut self,
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
            if self.exclude_globset.is_match(&path_without_prefix) {
                // ignore excluded paths
            } else if file_type.is_dir() {
                self.dirs.push(path_without_prefix);
                self.log_progress(None);
                self.visit_dir_tree(dir_entry.path(), prefix)?
            } else if file_type.is_symlink() {
                let target = fs::read_link(dir_entry.path())?;
                self.symlinks.push((path_without_prefix, target));
                self.log_progress(None);
            } else if file_type.is_file() {
                let md = dir_entry.metadata()?;
                let mut total_hashed = 0_u64;
                self.files.push(FileMetaData {
                    path: path_without_prefix,
                    len: md.len(),
                    modified: md.modified()?.into(),
                    hash: concat_hash(dir_entry.path(), |len| {
                        total_hashed += len;
                        self.log_progress(Some((total_hashed, md.len())));
                    })?,
                });
                self.log_progress(None);
            }
        }
        Ok(())
    }
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

fn main() -> Result<(), io::Error> {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        let dir = path::Path::new(args[1].as_str());
        let exclude_set = if args.len() > 2 {
            args[2..].iter().map(|x| x.clone()).collect()
        } else {
            HashSet::new()
        };
        let mut dir_tree = DirTree::new(dir.clone().into(), exclude_set);

        dir_tree.visit_dir_tree(dir, &dir.clone())?;
        eprintln!();

        dir_tree.found_dirs = dir_tree.dirs.len();
        dir_tree.found_symlinks = dir_tree.symlinks.len();
        dir_tree.found_files = dir_tree.files.len();
        dir_tree.files_total_size = dir_tree.files.iter().map(|f| f.len).sum();

        dir_tree.dirs.sort_unstable();
        dir_tree.symlinks.sort_unstable();
        dir_tree.files.sort_unstable();

        println!("{}", serde_json::to_string_pretty(&dir_tree).unwrap());

        Ok(())
    } else {
        eprintln!(
            "Usage:

    chumtree dir-tree-path exclude-glob-pattern* > chumtree.json

For a dir tree in 'dir-tree-path', output a JSON file with all the dirs,
all the symlinks and all the files with their checksum, size & mtime.

Use zero or more 'exclude-glob-pattern' to exclude files or dirs that match
the glob patterns; example: use '.DS_Store ._*' to exclude macOS folder settings 
and AppleDouble resource fork files.
See https://docs.rs/globset/0.4/globset/#syntax for the glob pattern syntax.
"
        );
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "command line arguments are missing",
        ))
    }
}
