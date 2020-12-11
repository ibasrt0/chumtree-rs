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

use std::collections::HashSet;
use std::env;
use std::io;
use std::path;
use chumtree::DirTree;

fn main() -> Result<(), io::Error> {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        let dir = path::Path::new(args[1].as_str());
        let exclude_set = if args.len() > 2 {
            args[2..].iter().map(|x| x.clone()).collect()
        } else {
            HashSet::new()
        };

        let mut dir_tree = DirTree::new(dir.clone().into(), exclude_set)
            .or_else(|e| Err(io::Error::new(io::ErrorKind::InvalidInput, e.to_string())))?;

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
the glob patterns; for example: use '.DS_Store' and '._*' to exclude macOS
folder settings and AppleDouble resource fork files.
See https://docs.rs/globset/0.4/globset/#syntax for the glob pattern syntax.
"
        );
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "command line arguments are missing",
        ))
    }
}
