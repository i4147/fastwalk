use pyo3::exceptions::{PyOSError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyAnyMethods, PyModule, PyString};
use std::path::{Path, PathBuf};

// Directories to skip during traversal
const SKIP_DIRS: &[&str] = &[".git", "__pycache__", ".mypy_cache", "lazy"];

// Bound API: return a Bound<'py, PyAny> instead of &'py PyAny.
fn to_py_path<'py>(py: Python<'py>, path: PathBuf) -> PyResult<Bound<'py, PyAny>> {
    let pathlib = py.import("pathlib")?;
    // PathBuf implements IntoPyObject, so we can pass it directly.
    pathlib.call_method1("Path", (path,))
}

#[derive(Clone)]
struct PathInput(PathBuf);

impl<'py> FromPyObject<'py> for PathInput {
    // Bound API: extract takes Bound<'py, PyAny>, not &'py PyAny.
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        // Try plain str first.
        if let Ok(s) = obj.extract::<String>() {
            return Ok(PathInput(PathBuf::from(s)));
        }

        // Try os.PathLike / pathlib.Path via __fspath__, falling back to __str__.
        if let Ok(path_str) = obj.call_method0("__fspath__") {
            if let Ok(s) = path_str.extract::<String>() {
                return Ok(PathInput(PathBuf::from(s)));
            }
        }

        if let Ok(path_str) = obj.call_method0("__str__") {
            if let Ok(s) = path_str.extract::<String>() {
                return Ok(PathInput(PathBuf::from(s)));
            }
        }

        Err(PyTypeError::new_err(
            "Expected a string or pathlib.Path object",
        ))
    }
}

impl From<PathInput> for PathBuf {
    fn from(input: PathInput) -> Self {
        input.0
    }
}

/// Helper function to filter entries based on skip list and symlink preference
fn should_skip_entry(entry: &jwalk::DirEntry<((), ())>, skip_symlinks: bool) -> bool {
    let name = entry.file_name();
    if let Some(name_str) = name.to_str() {
        if SKIP_DIRS.contains(&name_str) {
            return true;
        }
    }
    if skip_symlinks && entry.file_type().is_symlink() {
        return true;
    }
    false
}

#[pyfunction]
// Bound API: return Vec<Bound<'py, PyAny>>; tie 'py to Python<'py>.
fn walk<'py>(
    py: Python<'py>,
    path: PathInput,
    follow_links: Option<bool>,
) -> PyResult<Vec<Bound<'py, PyAny>>> {
    let follow = follow_links.unwrap_or(false);
    let path_buf: PathBuf = path.into();

    let walker = jwalk::WalkDir::new(&path_buf)
        .follow_links(follow)
        .process_read_dir(move |_depth: Option<usize>, _path: &Path, _state: &mut (), entries: &mut Vec<Result<jwalk::DirEntry<((), ())>, jwalk::Error>>| {
            entries.retain(|entry| match entry {
                Ok(e) => !should_skip_entry(e, !follow),
                Err(_) => false,
            });
        });

    let mut results = Vec::new();

    for entry in walker {
        match entry {
            Ok(e) => results.push(to_py_path(py, e.path())?),
            Err(e) => {
                return Err(PyOSError::new_err(format!(
                    "Error walking directory: {}",
                    e
                )));
            }
        }
    }
    Ok(results)
}

#[pyfunction]
fn walk_files<'py>(
    py: Python<'py>,
    path: PathInput,
    follow_links: Option<bool>,
) -> PyResult<Vec<Bound<'py, PyAny>>> {
    let follow = follow_links.unwrap_or(false);
    let path_buf: PathBuf = path.into();

    let walker = jwalk::WalkDir::new(&path_buf)
        .follow_links(follow)
        .process_read_dir(move |_depth: Option<usize>, _path: &Path, _state: &mut (), entries: &mut Vec<Result<jwalk::DirEntry<((), ())>, jwalk::Error>>| {
            entries.retain(|entry| match entry {
                Ok(e) => !should_skip_entry(e, !follow),
                Err(_) => false,
            });
        });

    let mut results = Vec::new();

    for entry in walker {
        match entry {
            Ok(e) => {
                if e.file_type().is_file() {
                    results.push(to_py_path(py, e.path())?);
                }
            }
            Err(e) => {
                return Err(PyOSError::new_err(format!(
                    "Error walking directory: {}",
                    e
                )));
            }
        }
    }
    Ok(results)
}

#[pyfunction]
fn walk_dirs<'py>(
    py: Python<'py>,
    path: PathInput,
    follow_links: Option<bool>,
) -> PyResult<Vec<Bound<'py, PyAny>>> {
    let follow = follow_links.unwrap_or(false);
    let path_buf: PathBuf = path.into();

    let walker = jwalk::WalkDir::new(&path_buf)
        .follow_links(follow)
        .process_read_dir(move |_depth: Option<usize>, _path: &Path, _state: &mut (), entries: &mut Vec<Result<jwalk::DirEntry<((), ())>, jwalk::Error>>| {
            entries.retain(|entry| match entry {
                Ok(e) => !should_skip_entry(e, !follow),
                Err(_) => false,
            });
        });

    let mut results = Vec::new();

    for entry in walker {
        match entry {
            Ok(e) => {
                if e.file_type().is_dir() {
                    results.push(to_py_path(py, e.path())?);
                }
            }
            Err(e) => {
                return Err(PyOSError::new_err(format!(
                    "Error walking directory: {}",
                    e
                )));
            }
        }
    }
    Ok(results)
}

#[pyclass]
// Removed #[derive(Clone)] — Py<PyAny> is no longer Clone.
struct Entry {
    #[pyo3(get)]
    path: Py<PyAny>,
    #[pyo3(get)]
    is_file: bool,
    #[pyo3(get)]
    is_dir: bool,
    #[pyo3(get)]
    is_symlink: bool,
    #[pyo3(get)]
    depth: usize,
}

#[pyfunction]
fn walk_with_metadata(
    py: Python<'_>,
    path: PathInput,
    follow_links: Option<bool>,
    max_depth: Option<usize>,
    min_depth: Option<usize>,
) -> PyResult<Vec<Entry>> {
    let follow = follow_links.unwrap_or(false);
    let path_buf: PathBuf = path.into();
    let mut walker = jwalk::WalkDir::new(&path_buf)
        .follow_links(follow)
        .process_read_dir(move |_depth: Option<usize>, _path: &Path, _state: &mut (), entries: &mut Vec<Result<jwalk::DirEntry<((), ())>, jwalk::Error>>| {
            entries.retain(|entry| match entry {
                Ok(e) => !should_skip_entry(e, !follow),
                Err(_) => false,
            });
        });

    if let Some(max) = max_depth {
        walker = walker.max_depth(max);
    }
    if let Some(min) = min_depth {
        walker = walker.min_depth(min);
    }

    let mut results = Vec::new();

    for entry in walker {
        match entry {
            Ok(e) => {
                let file_type = e.file_type();
                // Bound -> Py<PyAny> via .unbind().
                let py_path: Py<PyAny> = to_py_path(py, e.path())?.unbind();
                results.push(Entry {
                    path: py_path,
                    is_file: file_type.is_file(),
                    is_dir: file_type.is_dir(),
                    is_symlink: file_type.is_symlink(),
                    depth: e.depth(),
                });
            }
            Err(e) => {
                return Err(PyOSError::new_err(format!(
                    "Error walking directory: {}",
                    e
                )));
            }
        }
    }
    Ok(results)
}

#[pyfunction]
fn walk_parallel<'py>(
    py: Python<'py>,
    path: PathInput,
    follow_links: Option<bool>,
    num_threads: Option<usize>,
) -> PyResult<Vec<Bound<'py, PyAny>>> {
    let follow = follow_links.unwrap_or(false);
    let path_buf: PathBuf = path.into();
    let threads = num_threads.unwrap_or(0);

    let walker = jwalk::WalkDir::new(&path_buf)
        .follow_links(follow)
        .process_read_dir(move |_depth: Option<usize>, _path: &Path, _state: &mut (), entries: &mut Vec<Result<jwalk::DirEntry<((), ())>, jwalk::Error>>| {
            entries.retain(|entry| match entry {
                Ok(e) => !should_skip_entry(e, !follow),
                Err(_) => false,
            });
        })
        .parallelism(jwalk::Parallelism::RayonNewPool(threads));

    let mut results = Vec::new();

    for entry in walker {
        match entry {
            Ok(e) => results.push(to_py_path(py, e.path())?),
            Err(e) => {
                return Err(PyOSError::new_err(format!(
                    "Error walking directory: {}",
                    e
                )));
            }
        }
    }
    Ok(results)
}

#[pymodule]
// Bound API: the module fn takes &Bound<'_, PyModule>, not (py, &PyModule).
fn fastwalk(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(walk, m)?)?;
    m.add_function(wrap_pyfunction!(walk_files, m)?)?;
    m.add_function(wrap_pyfunction!(walk_dirs, m)?)?;
    m.add_function(wrap_pyfunction!(walk_with_metadata, m)?)?;
    m.add_function(wrap_pyfunction!(walk_parallel, m)?)?;
    m.add_class::<Entry>()?;
    Ok(())
}