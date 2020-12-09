chumtree-rs
===========

A [rust](https://www.rust-lang.org/) program that computes the **C**hecks**UM** 
of a directory **TREE**.

For a dir tree, output a JSON file with all the dirs, all the symlinks and 
all the files with their checksum, size & mtime.

The intended uses cases are for detecting bit rot and verifying copies of dir trees.

The hash function used as checksum is the very fast [SeaHash]() but
[warning: it is not a cryptographic function](https://docs.rs/seahash/4.0.1/seahash/#a-word-of-warning).

Unix and Windows permissions are ignored, to be able to compare checksum files 
between copies in filesystems with different capabilities.

To deal with macOS quirks:
-  `.DS_Store` and AppleDouble files (`._*`) are
filtered 
- unicode code points in the file names are recomposed, using
[Unicode Normal Form](https://en.wikipedia.org/wiki/Unicode_equivalence#Normal_forms)
Canonical Composition (NFC) to reverse the decomposition (NFD) imposed by macOS.

Currently, for simplicity, neither features are optional but probably they
should be (maybe this will be fixed in the future).

For a given directory tree, `chumtree` outputs a pretty printed JSON that it is
easily comparable with, for example, `diff`. The content of the JSON is:
- invocation timestamp
- the base directory
- all files total size in bytes
- relative path of all the directories
- relative path of all the symlinks and they targets
- relative path of all the files
- for each file:
  - size in bytes
  - time of last modification (mtime)
  - a checksum using a concatenate 64 bits hash for each 1 MiB block

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
Usage example:
```
$ chumtree test_dirtree > test_dirtree.chumtree.json
     0 dirs,      1 symlinks,      5 files found                                          
```
Output example, content of `test_dirtree.chumtree.json`:
```json
{
  "timestamp": "2020-11-26T12:55:35.837246Z",
  "base_dir": "test_dirtree",
  "found_dirs": 0,
  "found_symlinks": 1,
  "found_files": 5,
  "files_total_size": 15732736,
  "dirs": [],
  "symlinks": [
    [
      "symlink_test",
      "rand4096"
    ]
  ],
  "files": [
    {
      "path": "aaa 123",
      "len": 0,
      "modified": "2020-11-20T13:29:51.877186453Z",
      "hash": ""
    },
    {
      "path": "bbb\\",
      "len": 0,
      "modified": "2020-11-20T13:31:06.578608253Z",
      "hash": ""
    },
    {
      "path": "ccc\n123",
      "len": 0,
      "modified": "2020-11-20T13:32:01.004653136Z",
      "hash": ""
    },
    {
      "path": "rand15MiB",
      "len": 15728640,
      "modified": "2020-11-26T12:31:04.174375782Z",
      "hash": "B140D6CC95AC720DDFBE1FEED038EBD5E360B9BA95D97F0FAECCBDDDFB9EFD3AD5F3C2E55A7CB57BD69351A9C80C0D8F0DFCDA27D418E6CF110F7698F23AF37D7FD5F89163C2A7520A0D515A2673DE3DCEE5A34611FC92C09CAB692DAE45B0588CD5E28CA3356BB7C29111A3C3DC6A9D875AC0A2"
    },
    {
      "path": "rand4096",
      "len": 4096,
      "modified": "2020-11-26T12:30:04.251508553Z",
      "hash": "3C719153C8749D37"
    }
  ]
}
```

License
-------
[AGPLv3](http://www.gnu.org/licenses/agpl-3.0.html)
