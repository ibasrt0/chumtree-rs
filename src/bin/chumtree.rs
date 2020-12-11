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

use chumtree::ChumtreeFile;
use std::env;
use std::io;
use std::path;

const USAGE_TEXT: &str = "Usage:

  chumtree dir-tree-path exclude-glob-pattern* > chumtree.json

For a dir tree in 'dir-tree-path', output a JSON file with all the dirs,
all the symlinks and all the files with their checksum, size & mtime.

Use zero or more 'exclude-glob-pattern' to exclude files or dirs that match
the glob patterns; for example: use '.DS_Store' and '._*' to exclude macOS
folder settings and AppleDouble resource fork files.
See https://docs.rs/globset/0.4/globset/#syntax for the glob pattern syntax.
";

fn main() -> Result<(), io::Error> {
    if let Some(dir) = env::args().nth(1) {
        let dir = path::Path::new(dir.as_str());
        let options = chumtree::Options::new(dir.clone().into(), env::args().skip(2))
            .or_else(|e| Err(io::Error::new(io::ErrorKind::InvalidInput, e.to_string())))?;
        eprintln!(
            "base_dir: {:?}, exclude_set: {:?}",
            options.base_dir, options.exclude_set
        );
        let mut summary = chumtree::Summary::default();
        let mut dir_tree = chumtree::DirTree::default();

        chumtree::visit_dir_tree(&options, &mut summary, &mut dir_tree, dir, &dir.clone())?;
        eprintln!(
            "\r{:>6} dirs, {:>6} symlinks, {:>6} files found, {} bytes all files total size",
            summary.found_dirs,
            summary.found_symlinks,
            summary.found_files,
            summary.files_total_size
        );

        dir_tree.dirs.sort_unstable();
        dir_tree.symlinks.sort_unstable();
        dir_tree.files.sort_unstable();

        println!(
            "{}",
            serde_json::to_string_pretty(&ChumtreeFile {
                timestamp: chrono::offset::Utc::now(),
                options,
                summary,
                dir_tree
            })
            .or_else(|e| Err(io::Error::new(io::ErrorKind::InvalidData, e.to_string())))?
        );

        Ok(())
    } else {
        eprintln!("{}", USAGE_TEXT);
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "command line arguments are missing",
        ))
    }
}
