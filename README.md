chumtree-rs
===========

A [rust](https://www.rust-lang.org/) program that computes the **C**hecks**UM** 
of a directory **TREE**.

The intended uses cases are for detecting bit rot and verifying copies of dir
trees, so the program generates a JSON output file with all the dirs, all the
symlinks and all the files with their checksum, size & mtime. Unix and Windows
permissions are ignored, to be able to compare between copies in filesystems
with different capabilities. The output JSON file tries to be easily comparable
with, for example, `diff`.

The checksum is a concatenate 64 bits hash for each 1 MiB block. The hash
function used as checksum is the very fast [SeaHash]() but **WARNING**:
[it is not a cryptographic function](https://docs.rs/seahash/4.0.1/seahash/#a-word-of-warning).

To reverse the [unicode decomposition](https://en.wikipedia.org/wiki/Unicode_equivalence#Normal_forms) imposed by macOS unicode code points in the file names are recomposed, using
[Unicode Normal Form](https://en.wikipedia.org/wiki/Unicode_equivalence#Normal_forms)
Canonical Composition (NFC).  
Currently, for simplicity, this  features is not optional but it should be,
this will be fixed in the future.

Install
-------
Build from the source:
- Install the [rust toolchain](https://www.rust-lang.org/tools/install) in order
  to have cargo installed.
- `git clone` or download and extract the source file in `chumtree-rs`
   directory.
- Run `cd chumtree-rs; cargo build --release`
- Copy `chumtree-rs/target/release/chumtree` binary to a directory listed in the
  `PATH` env var. 

Usage
-----
```
$ chumtree dir-tree-path exclude-glob-pattern* > chumtree.json
```

For a dir tree in `dir-tree-path`, output a JSON file with all the dirs,
all the symlinks and all the files with their checksum, size & mtime.

Use zero or more `exclude-glob-pattern` to exclude files or dirs that match
the glob patterns; for example: use `**/.DS_Store` and `**/._*` to exclude macOS
folder settings and AppleDouble resource fork files.  
See https://docs.rs/globset/0.4/globset/#syntax for the glob pattern syntax.

Example
-------
```
$ chumtree test_dirtree "**/.DS_Store" > test_dirtree.chumtree.json
base_dir: "test_dirtree/", exclude_set: {"**/.DS_Store"}
     0 dirs,      1 symlinks,      5 files found, 15732736 bytes all files total size     
```

Output example, content of `test_dirtree.chumtree.json`:
```json
{
  "aaa 123": {
    "File": {
      "len": 0,
      "mtime": "2020-11-20T13:29:51.877186453Z",
      "hash": ""
    }
  },
  "bbb\\": {
    "File": {
      "len": 0,
      "mtime": "2020-11-20T13:31:06.578608253Z",
      "hash": ""
    }
  },
  "ccc\n123": {
    "File": {
      "len": 0,
      "mtime": "2020-11-20T13:32:01.004653136Z",
      "hash": ""
    }
  },
  "rand15MiB": {
    "File": {
      "len": 15728640,
      "mtime": "2020-11-26T12:31:04.174375782Z",
      "hash": "B140D6CC95AC720DDFBE1FEED038EBD5E360B9BA95D97F0FAECCBDDDFB9EFD3AD5F3C2E55A7CB57BD69351A9C80C0D8F0DFCDA27D418E6CF110F7698F23AF37D7FD5F89163C2A7520A0D515A2673DE3DCEE5A34611FC92C09CAB692DAE45B0588CD5E28CA3356BB7C29111A3C3DC6A9D875AC0A2"
    }
  },
  "rand4096": {
    "File": {
      "len": 4096,
      "mtime": "2020-11-26T12:30:04.251508553Z",
      "hash": "3C719153C8749D37"
    }
  },
  "symlink_test": {
    "Symlink": {
      "target": "rand4096"
    }
  }
}
```

License
-------
[AGPLv3](http://www.gnu.org/licenses/agpl-3.0.html)
