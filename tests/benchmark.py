import os
import time
import fastwalk
from pathlib import Path

SAMPLE_DIR = Path.home()


def benchmark_walk(path: str = SAMPLE_DIR):
    print(f"Benchmarking walk on: {path}")

    start = time.time()
    results = fastwalk.walk(path)
    elapsed = time.time() - start

    print(f"Found {len(results)} entries in {elapsed:.4f} seconds")
    print(f"Speed: {len(results) / elapsed:.2f} entries/second")
    return results


def benchmark_walk_files(path: str = SAMPLE_DIR):
    print(f"\nBenchmarking walk_files on: {path}")

    start = time.time()
    results = fastwalk.walk_files(path)
    elapsed = time.time() - start

    print(f"Found {len(results)} files in {elapsed:.4f} seconds")
    print(f"Speed: {len(results) / elapsed:.2f} files/second")
    return results


def benchmark_walk_with_metadata(path: str = SAMPLE_DIR):
    print(f"\nBenchmarking walk_with_metadata on: {path}")

    start = time.time()
    results = fastwalk.walk_with_metadata(path, max_depth=5)
    elapsed = time.time() - start

    print(f"Found {len(results)} entries in {elapsed:.4f} seconds")
    print(f"Speed: {len(results) / elapsed:.2f} entries/second")

    if results:
        entry = results[0]
        print(f"\nExample entry:")
        print(f"  Path: {entry.path}")
        print(f"  Is file: {entry.is_file}")
        print(f"  Is dir: {entry.is_dir}")
        print(f"  Depth: {entry.depth}")

    return results


def benchmark_walk_parallel(path: str = SAMPLE_DIR):
    print(f"\nBenchmarking walk_parallel on: {path}")

    start = time.time()
    results = fastwalk.walk_parallel(path, num_threads=4)
    elapsed = time.time() - start

    print(f"Found {len(results)} entries in {elapsed:.4f} seconds")
    print(f"Speed: {len(results) / elapsed:.2f} entries/second")
    return results


def compare_with_os_walk(path: str = SAMPLE_DIR):
    print(f"\nComparing with os.walk on: {path}")

    start = time.time()
    fastwalk_results = fastwalk.walk(path)
    fastwalk_time = time.time() - start

    start = time.time()
    os_walk_results = []
    for root, dirs, files in os.walk(path):
        os_walk_results.append(root)
        for d in dirs:
            os_walk_results.append(os.path.join(root, d))
        for f in files:
            os_walk_results.append(os.path.join(root, f))
    os_walk_time = time.time() - start

    print(f"fastwalk: {len(fastwalk_results)} entries in {fastwalk_time:.4f}s")
    print(f"os.walk:  {len(os_walk_results)} entries in {os_walk_time:.4f}s")
    print(f"Speedup:  {os_walk_time / fastwalk_time:.2f}x")


if __name__ == "__main__":
    test_path = SAMPLE_DIR

    print("=" * 60)
    print("FastWalk Benchmark Suite")
    print("=" * 60)

    benchmark_walk(test_path)
    benchmark_walk_files(test_path)
    benchmark_walk_with_metadata(test_path)
    benchmark_walk_parallel(test_path)
    compare_with_os_walk(test_path)

    print("\n" + "=" * 60)
    print("Benchmark complete!")
    print("=" * 60)

"""
============================================================
FastWalk Benchmark Suite
============================================================
Benchmarking walk on: /data/data/com.termux/files
Found 114907 entries in 2.7375 seconds
Speed: 41975.80 entries/second

Benchmarking walk_files on: /data/data/com.termux/files
Found 99278 files in 1.0563 seconds
Speed: 93986.95 files/second

Benchmarking walk_with_metadata on: /data/data/com.termux/files
Found 26947 entries in 0.2353 seconds
Speed: 114544.02 entries/second

Example entry:
  Path: /data/data/com.termux/files
  Is file: False
  Is dir: True
  Depth: 0

Benchmarking walk_parallel on: /data/data/com.termux/files
Found 114907 entries in 1.0897 seconds
Speed: 105444.71 entries/second

Comparing with os.walk on: /data/data/com.termux/files
fastwalk: 114907 entries in 1.1165s
os.walk:  152853 entries in 7.2689s
Speedup:  6.51x

============================================================
Benchmark complete!
============================================================

"""
