# fastwalk

Fast filesystem traversal for Python, backed by Rust's `jwalk`.

## Installation

Python 3.12+ and a Rust toolchain (Rust 1.83+) are required to build from source:

```sh
python -m pip install .
```

## Usage

All walkers accept a string or a string-valued `os.PathLike` object such as `pathlib.Path`. Like `rignore`, paths returned to Python are `pathlib.Path` objects. Unlike `rignore.walk`, these functions return a list rather than an iterator.

```python
from pathlib import Path
from fastwalk import walk_parallel, walk_with_metadata

for path in walk_parallel(Path("project"), num_threads=4):
    print(path.name)

for entry in walk_with_metadata("project", max_depth=2):
    print(entry.path, entry.depth, entry.is_dir)
```

- `walk(path, follow_links=None)` returns files and directories, including the root.
- `walk_files(path, follow_links=None)` returns only files.
- `walk_dirs(path, follow_links=None)` returns only directories, including the root.
- `walk_parallel(path, follow_links=None, num_threads=None)` returns files and directories using a dedicated thread pool. `None` or `0` chooses the thread count automatically.
- `walk_with_metadata(path, follow_links=None, max_depth=None, min_depth=None)` returns `Entry` objects. Each `Entry` has a `path` (`Path`), `is_file`, `is_dir`, `is_symlink`, and `depth`. The root has depth 0.

By default, all walkers skip symbolic links completely, including a symlink passed as the root. Set `follow_links=True` to include links and traverse linked directories; metadata for a followed link reports the target's file/directory type and `is_symlink=True`. Broken links and link loops can raise `OSError` when following links.

Directories named `.git`, `__pycache__`, `.mypy_cache`, `.myoy+cache`, or `.pytest_cache` are excluded before their contents are read, including when one is supplied as the root. Other hidden children are also skipped, preserving `jwalk`'s default behavior. An explicitly supplied hidden root is still walked unless it is one of the excluded directories or a symlink with link-following disabled. These exclusions are name-based; fastwalk does not apply `.gitignore` rules. Other traversal errors raise `OSError`.

## Development

In a virtual environment, install `maturin` and `pytest`, then run:

```sh
maturin develop
python -m pytest
```

For the Android cross-build configuration used by this project, see `build.sh`.
