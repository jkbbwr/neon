#!/usr/bin/env python3
# /// script
# dependencies = [
#   "rich",
# ]
# ///

import argparse
import os
import shutil
import subprocess
import sys

def main():
    parser = argparse.ArgumentParser(
        description="Run all benchmark suites in the neon2 repository.",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter
    )
    parser.add_argument("--fast-only", action="store_true", help="Only run languages within 5x the performance of C (based on cache).")
    parser.add_argument("--clear-cache", action="store_true", help="Clear the benchmark cache.")
    parser.add_argument("--runs", type=int, default=1, help="Number of runs per language to average.")
    parser.add_argument("--only", type=str, help="Comma-separated list of languages to run (fuzzy matched).")
    parser.add_argument("--historic", action="store_true", help="Compare this run to the last run for each language.")
    parser.add_argument("--only-bench", type=str, help="Comma-separated list of benchmarks to run (fuzzy matched, e.g., 'word', 'n-body').")
    parser.add_argument("--hide-compilation", action="store_true", help="Hide compilation output unless there is an error.")

    args, unknown = parser.parse_known_args()

    # Reconstruct the arguments to forward to individual benchmark runners
    forwarded_args = []
    if args.fast_only:
        forwarded_args.append("--fast-only")
    if args.clear_cache:
        forwarded_args.append("--clear-cache")
    if args.runs is not None:
        forwarded_args.extend(["--runs", str(args.runs)])
    if args.only is not None:
        forwarded_args.extend(["--only", args.only])
    if args.historic:
        forwarded_args.append("--historic")
    if args.hide_compilation:
        forwarded_args.append("--hide-compilation")
    
    forwarded_args.extend(unknown)

    # Detect the script's directory and the available benchmark suites
    script_dir = os.path.dirname(os.path.abspath(__file__))
    candidate_benchmarks = ["binary-trees", "brainfuck", "n-body", "word-frequency"]
    available_benchmarks = []
    for b in candidate_benchmarks:
        bench_dir = os.path.join(script_dir, b)
        if os.path.isdir(bench_dir) and os.path.exists(os.path.join(bench_dir, "run_bench.py")):
            available_benchmarks.append(b)

    if not available_benchmarks:
        print("Error: No benchmark suites with 'run_bench.py' found.", file=sys.stderr)
        sys.exit(1)

    # Filter benchmark suites if --only-bench is supplied
    benchmarks_to_run = []
    if args.only_bench:
        patterns = [p.strip().lower() for p in args.only_bench.split(",") if p.strip()]
        for pattern in patterns:
            matched = False
            for b in available_benchmarks:
                if pattern in b.lower():
                    if b not in benchmarks_to_run:
                        benchmarks_to_run.append(b)
                    matched = True
            if not matched:
                print(f"Warning: No benchmark directory matched filter '{pattern}'", file=sys.stderr)
    else:
        benchmarks_to_run = available_benchmarks

    if not benchmarks_to_run:
        print("Error: No benchmarks selected to run.", file=sys.stderr)
        sys.exit(1)

    # Set up console output formatting (using Rich if available)
    try:
        from rich.console import Console
        console = Console()
        has_rich = True
    except ImportError:
        console = None
        has_rich = False

    # Run each benchmark suite sequentially
    total_suites = len(benchmarks_to_run)
    for i, b in enumerate(benchmarks_to_run, start=1):
        bench_dir = os.path.join(script_dir, b)
        run_bench_script = os.path.join(bench_dir, "run_bench.py")

        header = f"=== [{i}/{total_suites}] Running Benchmark Suite: {b} ==="
        
        # Decide execution runner
        if shutil.which("uv"):
            cmd = ["uv", "run", "run_bench.py"] + forwarded_args
        else:
            cmd = [sys.executable, "run_bench.py"] + forwarded_args

        if has_rich:
            console.print(f"\n[bold green]{header}[/bold green]", highlight=False)
            console.print(f"[dim]Command: {' '.join(cmd)}[/dim]\n")
        else:
            print(f"\n{header}")
            print(f"Command: {' '.join(cmd)}\n")

        try:
            res = subprocess.run(cmd, cwd=bench_dir)
            if res.returncode != 0:
                msg = f"Benchmark suite '{b}' failed with exit code {res.returncode}"
                if has_rich:
                    console.print(f"\n[bold red]Error: {msg}[/bold red]\n")
                else:
                    print(f"\nError: {msg}\n", file=sys.stderr)
        except Exception as e:
            msg = f"Failed to run benchmark suite '{b}': {e}"
            if has_rich:
                console.print(f"\n[bold red]Error: {msg}[/bold red]\n")
            else:
                print(f"\nError: {msg}\n", file=sys.stderr)

if __name__ == "__main__":
    main()
