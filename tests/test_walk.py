import os
from pathlib import Path

import pytest

import fastwalk

WALKERS = (
    fastwalk.walk,
    fastwalk.walk_files,
    fastwalk.walk_dirs,
    fastwalk.walk_with_metadata,
    fastwalk.walk_parallel,
)
SKIPPED_DIRS = (".git", "__pycache__", ".mypy_cache", ".myoy+cache", ".pytest_cache")


def paths(walker, root, **kwargs):
    results = walker(root, **kwargs)
    assert isinstance(results, list)
    if walker is fastwalk.walk_with_metadata:
        assert all(isinstance(entry, fastwalk.Entry) for entry in results)
        results = [entry.path for entry in results]
    assert all(isinstance(path, Path) for path in results)
    return set(results)


def symlink(link, target, *, directory=False):
    try:
        link.symlink_to(target, target_is_directory=directory)
    except (OSError, NotImplementedError) as error:
        pytest.skip(f"symlinks are unavailable: {error}")


@pytest.fixture
def tree(tmp_path):
    root = tmp_path / "root"
    root.mkdir()
    (root / "file.txt").write_text("file")
    (root / ".hidden-file").write_text("hidden")
    (root / ".hidden-dir").mkdir()
    (root / ".hidden-dir" / "hidden.txt").write_text("hidden")
    visible = root / "visible"
    visible.mkdir()
    (visible / "data.txt").write_text("data")
    # A file with the same name is not an excluded directory.
    (visible / "__pycache__").write_text("not a directory")
    similarly_named = root / "__pycache__-keep"
    similarly_named.mkdir()
    (similarly_named / "keep.txt").write_text("keep")
    for parent in (root, visible):
        for name in SKIPPED_DIRS:
            skipped = parent / name
            # visible/__pycache__ is a regular file, tested above.
            if skipped.is_file():
                continue
            skipped.mkdir()
            (skipped / "contents.txt").write_text("excluded")
    return root


@pytest.mark.parametrize("walker", WALKERS)
@pytest.mark.parametrize("input_type", [Path, str])
def test_all_walkers_return_paths_and_skip_directories(tree, walker, input_type):
    all_entries = {
        tree,
        tree / "file.txt",
        tree / "visible",
        tree / "visible" / "data.txt",
        tree / "visible" / "__pycache__",
        tree / "__pycache__-keep",
        tree / "__pycache__-keep" / "keep.txt",
    }
    if walker is fastwalk.walk_files:
        expected = {path for path in all_entries if path.is_file()}
    elif walker is fastwalk.walk_dirs:
        expected = {path for path in all_entries if path.is_dir()}
    else:
        expected = all_entries
    assert paths(walker, input_type(tree)) == expected


@pytest.mark.parametrize("walker", WALKERS)
@pytest.mark.parametrize("name", SKIPPED_DIRS)
def test_excluded_directory_as_root_is_not_walked(tree, walker, name):
    assert paths(walker, tree / name) == set()


@pytest.mark.parametrize("walker", WALKERS)
@pytest.mark.parametrize("name", SKIPPED_DIRS)
def test_excluded_names_are_not_excluded_when_root_is_a_file(tmp_path, walker, name):
    file = tmp_path / name
    file.write_text("not a directory")
    assert paths(walker, file) == (set() if walker is fastwalk.walk_dirs else {file})


@pytest.mark.parametrize("walker", WALKERS)
def test_exclusions_prune_directories_even_when_following_links(tmp_path, walker):
    root = tmp_path / "root"
    root.mkdir()
    (root / "keep.txt").write_text("keep")
    for name in SKIPPED_DIRS:
        directory = root / name
        directory.mkdir()
        # Following this dangling link would raise OSError if the directory
        # were opened instead of being pruned.
        symlink(directory / "broken", root / "missing")
    results = paths(walker, root, follow_links=True)
    expected = set()
    if walker is not fastwalk.walk_dirs:
        expected.add(root / "keep.txt")
    if walker is not fastwalk.walk_files:
        expected.add(root)
    assert results == expected


@pytest.mark.parametrize("walker", WALKERS)
def test_followed_directory_link_with_excluded_name_is_pruned(tmp_path, walker):
    root = tmp_path / "root"
    root.mkdir()
    target = tmp_path / "target"
    target.mkdir()
    (target / "data.txt").write_text("excluded")
    link = root / "__pycache__"
    symlink(link, target, directory=True)
    assert paths(walker, link, follow_links=True) == set()
    expected = set() if walker is fastwalk.walk_files else {root}
    assert paths(walker, root, follow_links=True) == expected


@pytest.mark.parametrize("walker", WALKERS)
def test_followed_file_link_with_excluded_name_is_not_pruned(tmp_path, walker):
    root = tmp_path / "root"
    root.mkdir()
    target = tmp_path / "target"
    target.write_text("included")
    link = root / "__pycache__"
    symlink(link, target)
    expected = set() if walker is fastwalk.walk_dirs else {link}
    assert paths(walker, link, follow_links=True) == expected


@pytest.fixture
def linked_tree(tmp_path):
    root = tmp_path / "root"
    root.mkdir()
    real_file = root / "file.txt"
    real_file.write_text("content")
    real_dir = root / "directory"
    real_dir.mkdir()
    (real_dir / "child.txt").write_text("child")
    symlink(root / "file-link", real_file)
    symlink(root / "dir-link", real_dir, directory=True)
    return root


@pytest.mark.parametrize("walker", WALKERS)
@pytest.mark.parametrize("follow_links", [None, False])
def test_symlinks_are_skipped_by_default(linked_tree, walker, follow_links):
    root = linked_tree
    expected = {
        root,
        root / "file.txt",
        root / "directory",
        root / "directory" / "child.txt",
    }
    if walker is fastwalk.walk_files:
        expected = {root / "file.txt", root / "directory" / "child.txt"}
    elif walker is fastwalk.walk_dirs:
        expected = {root, root / "directory"}
    assert paths(walker, root, follow_links=follow_links) == expected


@pytest.mark.parametrize("walker", WALKERS)
def test_follow_links_opt_in(linked_tree, walker):
    root = linked_tree
    expected = {
        root,
        root / "file.txt",
        root / "directory",
        root / "directory" / "child.txt",
        root / "file-link",
        root / "dir-link",
        root / "dir-link" / "child.txt",
    }
    if walker is fastwalk.walk_files:
        expected = {path for path in expected if path.is_file()}
    elif walker is fastwalk.walk_dirs:
        expected = {path for path in expected if path.is_dir()}
    assert paths(walker, root, follow_links=True) == expected


def test_followed_link_metadata_identifies_link_and_target(linked_tree):
    entries = {
        entry.path: entry for entry in fastwalk.walk_with_metadata(linked_tree, True)
    }
    file_link = entries[linked_tree / "file-link"]
    dir_link = entries[linked_tree / "dir-link"]
    child = entries[linked_tree / "dir-link" / "child.txt"]
    assert (
        file_link.is_symlink,
        file_link.is_file,
        file_link.is_dir,
        file_link.depth,
    ) == (True, True, False, 1)
    assert (dir_link.is_symlink, dir_link.is_file, dir_link.is_dir, dir_link.depth) == (
        True,
        False,
        True,
        1,
    )
    assert (child.is_symlink, child.is_file, child.is_dir, child.depth) == (
        False,
        True,
        False,
        2,
    )


@pytest.mark.parametrize("walker", WALKERS)
def test_dangling_links_are_skipped_but_following_them_raises(tmp_path, walker):
    root = tmp_path / "root"
    root.mkdir()
    broken = root / "broken"
    symlink(broken, root / "missing")
    assert broken not in paths(walker, root)
    assert paths(walker, broken) == set()
    assert paths(walker, str(broken) + os.sep) == set()
    with pytest.raises(OSError):
        walker(root, follow_links=True)
    with pytest.raises(OSError):
        walker(broken, follow_links=True)


@pytest.mark.parametrize("walker", WALKERS)
def test_root_symlinks_are_skipped_without_being_traversed(linked_tree, walker):
    link = linked_tree / "dir-link"
    assert paths(walker, link) == set()
    assert paths(walker, str(link) + os.sep) == set()
    assert paths(walker, linked_tree / "file-link") == set()


@pytest.mark.parametrize("walker", WALKERS)
def test_follow_root_symlinks(linked_tree, walker):
    dir_link = linked_tree / "dir-link"
    file_link = linked_tree / "file-link"
    expected_dir = {dir_link, dir_link / "child.txt"}
    if walker is fastwalk.walk_files:
        expected_dir = {dir_link / "child.txt"}
    elif walker is fastwalk.walk_dirs:
        expected_dir = {dir_link}
    assert paths(walker, dir_link, follow_links=True) == expected_dir
    expected_file = set() if walker is fastwalk.walk_dirs else {file_link}
    assert paths(walker, file_link, follow_links=True) == expected_file


@pytest.mark.parametrize("suffix", ["", os.sep, os.sep + "." + os.sep])
def test_followed_root_link_metadata_with_trailing_separators(linked_tree, suffix):
    link = linked_tree / "dir-link"
    entries = fastwalk.walk_with_metadata(str(link) + suffix, follow_links=True)
    root = next(entry for entry in entries if entry.depth == 0)
    assert root.path == link
    assert (root.is_symlink, root.is_file, root.is_dir) == (True, False, True)


@pytest.mark.parametrize("walker", WALKERS)
def test_symlink_loop_is_skipped_by_default(linked_tree, walker):
    loop = linked_tree / "loop"
    symlink(loop, linked_tree, directory=True)
    assert loop not in paths(walker, linked_tree)
    with pytest.raises(OSError):
        walker(linked_tree, follow_links=True)


def test_metadata_depth_limits_and_properties(tree):
    entries = {entry.path: entry for entry in fastwalk.walk_with_metadata(tree)}
    root = entries[tree]
    child = entries[tree / "visible" / "data.txt"]
    assert (root.is_file, root.is_dir, root.is_symlink, root.depth) == (
        False,
        True,
        False,
        0,
    )
    assert (child.is_file, child.is_dir, child.is_symlink, child.depth) == (
        True,
        False,
        False,
        2,
    )
    assert paths(fastwalk.walk_with_metadata, tree, max_depth=0) == {tree}
    assert paths(fastwalk.walk_with_metadata, tree, min_depth=1, max_depth=1) == {
        tree / "file.txt",
        tree / "visible",
        tree / "__pycache__-keep",
    }
    assert all(
        entry.depth == 2 for entry in fastwalk.walk_with_metadata(tree, min_depth=2)
    )


def test_parallel_thread_count_keeps_the_same_results(tree):
    expected = paths(fastwalk.walk, tree)
    for threads in (0, 1, 2, 4):
        assert paths(fastwalk.walk_parallel, tree, num_threads=threads) == expected


@pytest.mark.parametrize("walker", WALKERS)
def test_single_file_root_and_missing_root(tmp_path, walker):
    file = tmp_path / "file.txt"
    file.write_text("content")
    assert paths(walker, file) == (set() if walker is fastwalk.walk_dirs else {file})
    with pytest.raises(OSError):
        walker(tmp_path / "missing")


@pytest.mark.parametrize("walker", WALKERS)
def test_relative_and_pathlike_input(tmp_path, monkeypatch, walker):
    root = tmp_path / "root"
    root.mkdir()
    (root / "file.txt").write_text("content")
    monkeypatch.chdir(tmp_path)

    class CustomPath:
        def __fspath__(self):
            return "root"

    result = paths(walker, CustomPath())
    expected = {Path("root"), Path("root/file.txt")}
    if walker is fastwalk.walk_files:
        expected.remove(Path("root"))
    elif walker is fastwalk.walk_dirs:
        expected.remove(Path("root/file.txt"))
    assert result == expected


@pytest.mark.skipif(os.name != "posix", reason="requires POSIX byte paths")
def test_non_utf8_names_round_trip_as_paths(tmp_path):
    raw_path = os.fsencode(tmp_path) + b"/name-\xff"
    with open(raw_path, "wb") as file:
        file.write(b"content")
    expected = Path(os.fsdecode(raw_path))
    assert paths(fastwalk.walk_files, tmp_path) == {expected}
    assert paths(fastwalk.walk_files, str(expected)) == {expected}
    entries = fastwalk.walk_with_metadata(tmp_path)
    assert any(entry.path == expected and entry.is_file for entry in entries)
    assert os.fsencode(expected) == raw_path
