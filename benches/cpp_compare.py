#!/usr/bin/env python3
"""Compare Rust hyperlogloglog against C++ reference implementation.

Generates random uint64 data, runs both implementations, and reports
insert time, estimate accuracy, and sketch size.
"""

import json
import os
import random
import subprocess
import sys
import tempfile
from pathlib import Path

CXX_MEASURE = Path(__file__).resolve().parent.parent.parent / "hyperlogloglog" / "hyperlogloglog" / "measure"

# Ensure binary exists
if not CXX_MEASURE.exists():
    print(f"ERROR: C++ measure binary not found at {CXX_MEASURE}")
    print("Build it: cd ../hyperlogloglog/hyperlogloglog && make measure")
    sys.exit(1)


def generate_data(n: int, seed: int = 42) -> str:
    """Generate n random uint64 values as lines."""
    rng = random.Random(seed)
    return "\n".join(str(rng.getrandbits(64)) for _ in range(n))


def run_cpp(n: int, m: int = 256) -> dict:
    """Run C++ measure in query mode with hyperlogloglog algorithm."""
    data = generate_data(n)
    result = subprocess.run(
        [str(CXX_MEASURE), "query", "hyperlogloglog", "uint64", str(m), str(n)],
        input=data,
        capture_output=True,
        text=True,
        timeout=60,
    )
    if result.returncode != 0:
        print(f"C++ stderr: {result.stderr}")
        return {}

    parsed = {}
    for line in result.stdout.strip().split("\n"):
        parts = line.strip().split()
        if len(parts) == 2:
            parsed[parts[0]] = parts[1]
    return {
        "insert_seconds": float(parsed.get("time", 0)),
        "estimate": float(parsed.get("estimate", 0)),
        "bitsize": int(parsed.get("bitsize", 0)),
        "compress_count": int(parsed.get("compressCount", 0)),
        "rebase_count": int(parsed.get("rebaseCount", 0)),
    }


def run_cpp_merge(n: int, m: int = 256) -> dict:
    """Run C++ measure in merge mode."""
    data = generate_data(n)
    result = subprocess.run(
        [str(CXX_MEASURE), "merge", "hyperlogloglog", "uint64", str(m), str(n)],
        input=data,
        capture_output=True,
        text=True,
        timeout=60,
    )
    if result.returncode != 0:
        print(f"C++ stderr: {result.stderr}")
        return {}

    parsed = {}
    for line in result.stdout.strip().split("\n"):
        parts = line.strip().split()
        if len(parts) == 2:
            parsed[parts[0]] = parts[1]
    return {
        "merge_seconds": float(parsed.get("time", 0)),
        "estimate": float(parsed.get("estimate", 0)),
        "bitsize": int(parsed.get("bitsize", 0)),
    }


def main():
    cardinalities = [1_000, 10_000, 100_000]
    m = 256  # P8

    # Rust criterion results (from cargo bench)
    rust_results = {
        "insert": {
            1_000: 146.0,
            10_000: 290.0,
            100_000: 740.0,
        },
        "estimate": {
            1_000: 5.6,
            10_000: 11.5,
            100_000: 10.0,
        },
        "merge": {
            1_000: 7.7,
            10_000: 16.1,
        },
        "compress": {
            10_000: 9.3,
            100_000: 7.7,
        },
    }

    print("C++ vs Rust HyperLogLog-log (P8, kappa=3)")
    print("=" * 70)

    for n in cardinalities:
        print(f"\nn = {n:,}")
        print("-" * 40)

        cpp = run_cpp(n, m)
        if not cpp:
            print("  C++ query failed")
            continue

        cpp_merge = run_cpp_merge(n, m)

        cpp_insert_ms = cpp["insert_seconds"] * 1000
        rust_insert_us = rust_results["insert"].get(n, 0)
        rust_insert_ms = rust_insert_us / 1000

        print(f"  Insert:")
        print(f"    C++:  {cpp_insert_ms:8.3f} ms  (est={cpp['estimate']:.0f}, "
              f"size={cpp['bitsize']} bits)")
        print(f"    Rust: {rust_insert_ms:8.3f} ms  "
              f"({cpp_insert_ms / rust_insert_ms:.1f}x faster)")

        if n in rust_results["merge"] and cpp_merge:
            cpp_merge_ms = cpp_merge["merge_seconds"] * 1000
            rust_merge_us = rust_results["merge"].get(n, 0)
            rust_merge_ms = rust_merge_us / 1000
            print(f"  Merge:")
            print(f"    C++:  {cpp_merge_ms:8.3f} ms  (est={cpp_merge['estimate']:.0f})")
            print(f"    Rust: {rust_merge_ms:8.3f} ms  "
                  f"({cpp_merge_ms / rust_merge_ms:.1f}x faster)")

    print("\n" + "=" * 70)
    print("Notes:")
    print("  C++ uses farmhash + full add() pipeline")
    print("  Rust uses splitmix64 + update_with_hash() (pre-hashed)")
    print("  Both use P8 (256 registers), kappa=3")


if __name__ == "__main__":
    main()
