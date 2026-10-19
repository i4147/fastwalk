use jwalk::{DirEntry, Parallelism, WalkDir};
use pyo3::exceptions::PyOSError;
use pyo3::prelude::*;
use std::path::PathBuf;

type Item = Result<DirEntry<((), ())>, jwalk::Error>;

/// Default directory names to skip. A leading `*` means "ends with"
/// (so `*.dist-info` skips every `foo-1.0.dist-info` directory).
const DEFAULT_SKIP_DIRS: &[&str] = &[
    ".git",
    "__pycache__",
    "lazy",
    ".mypy_cache",
    ".pytest_cache",
    "*.dist-info",
];

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    All,
    Files,
    Dirs,
}

fn name_matches(name: &str, pattern: &str) -> bool {
    match pattern.strip_prefix('*') {
        Some(suffix) => name.ends_with(suffix),
        None => name == pattern,
    }
}

fn normalize_exts(exts: Option<Vec<String>>) -> Option<Vec<String>> {
    exts.map(|v| {
        v.into_iter()
            .map(|e| {
                let e = e.to_lowercase();
                if e.starts_with('.') {
                    e
                } else {
                    format!(".{e}")
                }
            })
            .collect()
    })
}

/// Shared walking logic used by all iterators.
struct Core {
    iter: Box<dyn Iterator<Item = Item>>,
    kind: Kind,
    exts: Option<Vec<String>>,
}

impl Core {
    fn new(
        path: PathBuf,
        kind: Kind,
        exts: Option<Vec<String>>,
        skip_dirs: Option<Vec<String>>,
        follow_links: bool,
        skip_symlinks: bool,
        max_depth: Option<usize>,
        min_depth: Option<usize>,
        parallelism: Option<Parallelism>,
    ) -> Self {
        let skip_dirs: Vec<String> =
            skip_dirs.unwrap_or_else(|| DEFAULT_SKIP_DIRS.iter().map(|s| s.to_string()).collect());

        let mut walker = WalkDir::new(path).follow_links(follow_links);
        if let Some(max) = max_depth {
            walker = walker.max_depth(max);
        }
        if let Some(min) = min_depth {
            walker = walker.min_depth(min);
        }
        if let Some(p) = parallelism {
            walker = walker.parallelism(p);
        }

        // Prune before descending, so skipped dirs are never read.
        let walker = walker.process_read_dir(move |_depth, _path, _state, children| {
            children.retain(|res| match res {
                Ok(e) => {
                    if skip_symlinks && e.path_is_symlink() {
                        return false;
                    }
                    if e.file_type().is_dir() {
                        let name = e.file_name().to_string_lossy();
                        if skip_dirs.iter().any(|p| name_matches(&name, p)) {
                            return false;
                        }
                    }
                    true
                }
                Err(_) => true, // let errors surface in the iterator
            });
        });

        Core {
            iter: Box::new(walker.into_iter()),
            kind,
            exts: normalize_exts(exts),
        }
    }

    fn accept(&self, e: &DirEntry<((), ())>) -> bool {
        let ft = e.file_type();
        match self.kind {
            Kind::Files if !ft.is_file() => return false,
            Kind::Dirs if !ft.is_dir() => return false,
            _ => {}
        }
        if let (Some(exts), true) = (&self.exts, self.kind != Kind::Dirs) {
            if ft.is_dir() {
                return false;
            }
            let name = e.file_name().to_string_lossy().to_lowercase();
            if !exts.iter().any(|x| name.ends_with(x.as_str())) {
                return false;
            }
        }
        true
    }

    fn next_entry(&mut self) -> PyResult<Option<DirEntry<((), ())>>> {
        while let Some(item) = self.iter.next() {
            match item {
                Ok(e) => {
                    if self.accept(&e) {
                        return Ok(Some(e));
                    }
                }
                Err(err) => {
                    return Err(PyOSError::new_err(format!(
                        "Error walking directory: {}",
                        err
                    )));
                }
            }
        }
        Ok(None)
    }
}

/// Lazy iterator yielding path strings.
#[pyclass(unsendable)]
struct PathIter {
    core: Core,
}

#[pymethods]
impl PathIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> PyResult<Option<String>> {
        Ok(self
            .core
            .next_entry()?
            .map(|e| e.path().to_string_lossy().to_string()))
    }
}

#[pyclass]
#[derive(Clone)]
struct Entry {
    #[pyo3(get)]
    path: String,
    #[pyo3(get)]
    is_file: bool,
    #[pyo3(get)]
    is_dir: bool,
    #[pyo3(get)]
    is_symlink: bool,
    #[pyo3(get)]
    depth: usize,
}

/// Lazy iterator yielding `Entry` objects.
#[pyclass(unsendable)]
struct EntryIter {
    core: Core,
}

#[pymethods]
impl EntryIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> PyResult<Option<Entry>> {
        Ok(self.core.next_entry()?.map(|e| {
            let ft = e.file_type();
            Entry {
                path: e.path().to_string_lossy().to_string(),
                is_file: ft.is_file(),
                is_dir: ft.is_dir(),
                is_symlink: e.path_is_symlink(),
                depth: e.depth(),
            }
        }))
    }
}

/// Walk everything (files + dirs). With `exts`, only matching files are yielded.
#[pyfunction]
#[pyo3(signature = (path, exts=None, skip_dirs=None, follow_links=false, skip_symlinks=true))]
fn walk(
    path: PathBuf,
    exts: Option<Vec<String>>,
    skip_dirs: Option<Vec<String>>,
    follow_links: bool,
    skip_symlinks: bool,
) -> PathIter {
    PathIter {
        core: Core::new(
            path,
            Kind::All,
            exts,
            skip_dirs,
            follow_links,
            skip_symlinks,
            None,
            None,
            None,
        ),
    }
}

#[pyfunction]
#[pyo3(signature = (path, exts=None, skip_dirs=None, follow_links=false, skip_symlinks=true))]
fn walk_files(
    path: PathBuf,
    exts: Option<Vec<String>>,
    skip_dirs: Option<Vec<String>>,
    follow_links: bool,
    skip_symlinks: bool,
) -> PathIter {
    PathIter {
        core: Core::new(
            path,
            Kind::Files,
            exts,
            skip_dirs,
            follow_links,
            skip_symlinks,
            None,
            None,
            None,
        ),
    }
}

/// Directories only (`exts` doesn't apply here).
#[pyfunction]
#[pyo3(signature = (path, skip_dirs=None, follow_links=false, skip_symlinks=true))]
fn walk_dirs(
    path: PathBuf,
    skip_dirs: Option<Vec<String>>,
    follow_links: bool,
    skip_symlinks: bool,
) -> PathIter {
    PathIter {
        core: Core::new(
            path,
            Kind::Dirs,
            None,
            skip_dirs,
            follow_links,
            skip_symlinks,
            None,
            None,
            None,
        ),
    }
}

#[pyfunction]
#[pyo3(signature = (
    path, exts=None, skip_dirs=None, follow_links=false, skip_symlinks=true,
    max_depth=None, min_depth=None
))]
fn walk_with_metadata(
    path: PathBuf,
    exts: Option<Vec<String>>,
    skip_dirs: Option<Vec<String>>,
    follow_links: bool,
    skip_symlinks: bool,
    max_depth: Option<usize>,
    min_depth: Option<usize>,
) -> EntryIter {
    EntryIter {
        core: Core::new(
            path,
            Kind::All,
            exts,
            skip_dirs,
            follow_links,
            skip_symlinks,
            max_depth,
            min_depth,
            None,
        ),
    }
}

#[pyfunction]
#[pyo3(signature = (
    path, exts=None, skip_dirs=None, follow_links=false, skip_symlinks=true,
    num_threads=None
))]
fn walk_parallel(
    path: PathBuf,
    exts: Option<Vec<String>>,
    skip_dirs: Option<Vec<String>>,
    follow_links: bool,
    skip_symlinks: bool,
    num_threads: Option<usize>,
) -> PathIter {
    PathIter {
        core: Core::new(
            path,
            Kind::All,
            exts,
            skip_dirs,
            follow_links,
            skip_symlinks,
            None,
            None,
            Some(Parallelism::RayonNewPool(num_threads.unwrap_or(0))),
        ),
    }
}

#[pymodule]
fn fastwalk(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(walk, m)?)?;
    m.add_function(wrap_pyfunction!(walk_files, m)?)?;
    m.add_function(wrap_pyfunction!(walk_dirs, m)?)?;
    m.add_function(wrap_pyfunction!(walk_with_metadata, m)?)?;
    m.add_function(wrap_pyfunction!(walk_parallel, m)?)?;
    m.add_class::<Entry>()?;
    m.add_class::<PathIter>()?;
    m.add_class::<EntryIter>()?;
    Ok(())
}
