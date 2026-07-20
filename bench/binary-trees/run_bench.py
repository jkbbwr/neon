#!/usr/bin/env python3
# /// script
# dependencies = [
#   "rich",
# ]
# ///

import subprocess
import time
import shutil
import os
import glob
import json
import argparse
from rich.console import Console
from rich.table import Table

def run_and_time(cmd):
    start = time.perf_counter()
    res = subprocess.run(cmd, capture_output=True, text=True)
    end = time.perf_counter()
    if res.returncode != 0:
        raise Exception(f"Command {' '.join(cmd)} failed with exit code {res.returncode}: {res.stderr}")
    if "Result: 68332206" not in res.stdout:
        raise Exception(f"Command {' '.join(cmd)} returned incorrect output: {res.stdout.strip()}")
    return end - start

def compile_and_show_output(name, cmd, cwd=None):
    console = Console()
    import sys
    hide_compilation = "--hide-compilation" in sys.argv

    if not hide_compilation:
        console.print(f"Compiling [cyan]{name}[/cyan]...")

    res = subprocess.run(cmd, capture_output=True, text=True, cwd=cwd)
    is_ok = (res.returncode == 0)

    if not is_ok:
        if hide_compilation:
            console.print(f"Compiling [cyan]{name}[/cyan]... [red]Failed[/red]")
        
        output_str = (res.stdout or "") + (res.stderr or "")
        output_str = output_str.strip() or "(No compiler output)"
        
        border = "=" * 60
        console.print(border)
        console.print(f"Compiler output for {name}:")
        console.print(output_str)
        console.print(border)
    else:
        if hide_compilation:
            console.print(f"Compiling [cyan]{name}[/cyan]... [green]OK[/green]")
        else:
            output_str = (res.stdout or "") + (res.stderr or "")
            output_str = output_str.strip() or "(No compiler output)"
            
            border = "=" * 60
            console.print(border)
            console.print(f"Compiler output for {name}:")
            console.print(output_str)
            console.print(border)
            
    return is_ok

def main():
    parser = argparse.ArgumentParser(description="Run binary-trees benchmarks.")
    parser.add_argument("--fast-only", action="store_true", help="Only run languages within 5x the performance of C (based on cache).")
    parser.add_argument("--clear-cache", action="store_true", help="Clear the benchmark cache.")
    parser.add_argument("--runs", type=int, default=1, help="Number of runs per language to average.")
    parser.add_argument("--only", type=str, help="Comma-separated list of languages to run (fuzzy matched).")
    parser.add_argument("--historic", action="store_true", help="Compare this run to the last run for each language.")
    parser.add_argument("--hide-compilation", action="store_true", help="Hide compilation output unless there is an error.")
    args = parser.parse_args()

    # Make sure we are in the script's directory
    script_dir = os.path.dirname(os.path.abspath(__file__))
    os.chdir(script_dir)

    cache_file = ".bench_cache.json"
    if args.clear_cache and os.path.exists(cache_file):
        os.remove(cache_file)

    cache = {}
    if os.path.exists(cache_file):
        try:
            with open(cache_file, "r") as f:
                cache = json.load(f)
        except Exception:
            pass

    # Normalize cache to the new format internally
    if isinstance(cache, dict) and "runs" not in cache:
        if cache:
            cache = {
                "runs": [
                    {
                        "timestamp": "legacy",
                        "results": cache
                    }
                ]
            }
        else:
            cache = {"runs": []}

    latest_cache = {}
    if isinstance(cache, dict) and "runs" in cache:
        for run in cache["runs"]:
            if isinstance(run, dict) and "results" in run:
                for k, v in run["results"].items():
                    latest_cache[k] = v

    only_patterns = []
    if args.only:
        only_patterns = [p.strip().lower() for p in args.only.split(",") if p.strip()]

    def matches_only(name):
        if not args.only:
            return True
        name_lower = name.lower()
        import re
        name_tokens = re.sub(r'[^a-z0-9+#]', ' ', name_lower).split()
        name_clean = re.sub(r'[^a-z0-9+#]', '', name_lower)
        
        aliases = {
            'cpp': 'c++',
            'csharp': 'c#',
        }
        
        for pattern in only_patterns:
            pattern = aliases.get(pattern, pattern)
            pattern_clean = re.sub(r'[^a-z0-9+#]', '', pattern)
            
            if pattern_clean == name_clean:
                return True
            if pattern in name_tokens or pattern_clean in name_tokens:
                return True
            if len(pattern_clean) >= 3:
                if any(token.startswith(pattern_clean) for token in name_tokens):
                    return True
                if pattern_clean in name_clean:
                    return True
        return False

    def should_run(name):
        if args.only and not matches_only(name):
            return False
        if not args.fast_only:
            return True
        if name == "C":
            return True
        if name not in latest_cache or "C" not in latest_cache:
            return True
        return latest_cache[name] <= 5.0 * latest_cache["C"]

    console = Console()
    console.print("[bold blue]Compiling binaries...[/bold blue]")
    
    # Compile C
    c_ok = False
    if should_run('C'):
        if shutil.which("gcc"):
            c_ok = compile_and_show_output("C", ["gcc", "-O3", "c/main.c", "-o", "c/bt"])
        else:
            console.print("[yellow]gcc not found. Cannot compile C.[/yellow]")

    # Compile C++
    cpp_ok = False
    if should_run('C++'):
        if shutil.which("g++"):
            cpp_ok = compile_and_show_output("C++", ["g++", "-O3", "cpp/main.cpp", "-o", "cpp/bt"])
        else:
            console.print("[yellow]g++ not found. Cannot compile C++.[/yellow]")

    # Compile Zig
    zig_ok = False
    if should_run('Zig'):
        if shutil.which("zig"):
            zig_ok = compile_and_show_output("Zig", ["zig", "build-exe", "-O", "ReleaseFast", "zig/main.zig", "-femit-bin=zig/bt"])
        else:
            console.print("[yellow]zig not found. Cannot compile Zig.[/yellow]")

    # Compile Rust
    rust_ok = False
    if should_run('Rust'):
        if shutil.which("cargo"):
            rust_ok = compile_and_show_output("Rust", ["cargo", "build", "--release", "--manifest-path", "rust/Cargo.toml"])
        else:
            console.print("[yellow]cargo not found. Cannot compile Rust.[/yellow]")

    # Compile Neon
    neon_ok = False
    if should_run('Neon'):
        if shutil.which("cargo"):
            neon_ok = compile_and_show_output("Neon", ["cargo", "run", "--release", "--manifest-path", "../../../Cargo.toml", "--bin", "neon", "--", "build"], cwd="neon")

    # Compile Go
    go_ok = False
    if should_run('Go'):
        if shutil.which("go"):
            go_ok = compile_and_show_output("Go", ["go", "build", "-o", "go/bt", "go/main.go"])

    # Compile Dart
    dart_ok = False
    if should_run('Dart'):
        if shutil.which("dart"):
            dart_ok = compile_and_show_output("Dart", ["dart", "compile", "exe", "dart/main.dart", "-o", "dart/bt"])
        else:
            console.print("[yellow]dart not found. Cannot compile Dart.[/yellow]")

    # Compile C# (.NET)
    dotnet_ok = False
    if should_run('C# (.NET)'):
        if shutil.which("dotnet"):
            dotnet_ok = compile_and_show_output("C# (.NET)", ["dotnet", "publish", "dotnet/binarytrees.csproj", "-c", "Release", "-o", "dotnet/build"])

    # Compile Java
    java_ok = False
    if should_run('Java'):
        if shutil.which("javac") and shutil.which("java"):
            java_ok = compile_and_show_output("Java", ["javac", "java/Main.java"])
        else:
            console.print("[yellow]Java compiler or runtime not found.[/yellow]")

    # Compile Haskell
    haskell_ok = False
    if should_run('Haskell'):
        if shutil.which("ghc"):
            haskell_ok = compile_and_show_output("Haskell", ["ghc", "-O2", "haskell/Main.hs", "-o", "haskell/bt"])
        else:
            console.print("[yellow]ghc not found. Cannot compile Haskell.[/yellow]")

    # Compile OCaml
    ocaml_ok = False
    if should_run('OCaml'):
        if shutil.which("ocamlopt"):
            ocaml_ok = compile_and_show_output("OCaml", ["ocamlopt", "-O3", "ocaml/main.ml", "-o", "ocaml/bt"])
        else:
            console.print("[yellow]ocamlopt not found. Cannot compile OCaml.[/yellow]")

    # Check interpreter runtimes
    python_ok = shutil.which("python3") is not None
    js_ok = shutil.which("node") is not None
    bun_ok = shutil.which("bun") is not None
    deno_ok = shutil.which("deno") is not None
    luajit_ok = shutil.which("luajit") is not None
    lua_ok = shutil.which("lua") is not None
    ruby_ok = shutil.which("ruby") is not None
    elixir_ok = shutil.which("elixir") is not None
    perl_ok = shutil.which("perl") is not None
    clojure_ok = shutil.which("clojure") is not None

    # Compile Java Native
    java_native_ok = False
    if should_run('Java (Native)'):
        if java_ok and shutil.which("native-image"):
            java_native_ok = compile_and_show_output("Java (Native)", ["native-image", "-cp", "java", "-O3", "Main", "-o", "java/bt_native"])

    # Compile Clojure Native
    clojure_native_ok = False
    if should_run('Clojure (Native)'):
        if clojure_ok and shutil.which("native-image") and shutil.which("java"):
            try:
                clojure_cp = subprocess.run(["clojure", "-Spath"], capture_output=True, text=True, check=True).stdout.strip()
                os.makedirs("clojure/classes", exist_ok=True)
                aot_ok = compile_and_show_output("Clojure AOT", ["java", "-Dclojure.compile.path=clojure/classes", "-cp", f"{clojure_cp}:clojure:clojure/classes", "clojure.lang.Compile", "main"])
                if aot_ok:
                    clojure_native_ok = compile_and_show_output("Clojure (Native)", ["native-image", "-cp", f"{clojure_cp}:clojure/classes", "-O3", "--no-fallback", "--initialize-at-build-time", "main", "-o", "clojure/bt_native"])
            except Exception as e:
                console.print(f"[red]Failed to compile Clojure (Native): {e}[/red]")

    targets = [
        {"name": "Haskell", "cmd": ["./haskell/bt"], "available": haskell_ok},
        {"name": "OCaml", "cmd": ["./ocaml/bt"], "available": ocaml_ok},
        {"name": "C", "cmd": ["./c/bt"], "available": c_ok},
        {"name": "C++", "cmd": ["./cpp/bt"], "available": cpp_ok},
        {"name": "Zig", "cmd": ["./zig/bt"], "available": zig_ok},
        {"name": "Rust", "cmd": ["./rust/target/release/binarytrees"], "available": rust_ok},
        {"name": "Go", "cmd": ["./go/bt"], "available": go_ok},
        {"name": "Dart", "cmd": ["./dart/bt"], "available": dart_ok},
        {"name": "C# (.NET)", "cmd": ["./dotnet/build/binarytrees"], "available": dotnet_ok},
        {"name": "Java (Native)", "cmd": ["./java/bt_native"], "available": java_native_ok},
        {"name": "JS (Bun)", "cmd": ["bun", "js/main.js"], "available": bun_ok},
        {"name": "JS (Deno)", "cmd": ["deno", "run", "js/main.js"], "available": deno_ok},
        {"name": "JS (Node)", "cmd": ["node", "js/main.js"], "available": js_ok},
        {"name": "TS (Bun)", "cmd": ["bun", "ts/main.ts"], "available": bun_ok},
        {"name": "TS (Deno)", "cmd": ["deno", "run", "ts/main.ts"], "available": deno_ok},
        {"name": "LuaJIT", "cmd": ["luajit", "lua/main.lua"], "available": luajit_ok},
        {"name": "Neon", "cmd": ["./neon/_neon/binarytrees"], "available": neon_ok},
        {"name": "Java (JVM)", "cmd": ["java", "-cp", "java", "Main"], "available": java_ok},
        {"name": "Ruby (YJIT)", "cmd": ["ruby", "--yjit", "ruby/main.rb"], "available": ruby_ok},
        {"name": "Lua", "cmd": ["lua", "lua/main.lua"], "available": lua_ok},
        {"name": "Clojure (Native)", "cmd": ["./clojure/bt_native"], "available": clojure_native_ok},
        {"name": "Python", "cmd": ["python3", "python/main.py"], "available": python_ok},
        {"name": "Ruby", "cmd": ["ruby", "ruby/main.rb"], "available": ruby_ok},
        {"name": "Elixir", "cmd": ["elixir", "elixir/main.exs"], "available": elixir_ok},
        {"name": "Clojure (JVM)", "cmd": ["clojure", "-M", "clojure/main.clj"], "available": clojure_ok},
        {"name": "Perl", "cmd": ["perl", "perl/main.pl"], "available": perl_ok},
    ]

    runs_str = f"{args.runs} runs" if args.runs > 1 else "1 run"
    console.print(f"\n[bold blue]Running benchmarks ({runs_str} each, taking average)...[/bold blue]")
    results = {}
    
    for t in targets:
        if not t["available"] or not should_run(t["name"]):
            results[t["name"]] = None
            continue
        
        console.print(f"Benchmarking [cyan]{t['name']}[/cyan]...")
        times = []
        try:
            for _ in range(args.runs):
                elapsed = run_and_time(t["cmd"])
                times.append(elapsed)
            results[t["name"]] = sum(times) / len(times)
        except Exception as e:
            console.print(f"[red]Error running {t['name']}: {e}[/red]")
            results[t["name"]] = None

    # Print results table
    c_time = results.get("C")
    
    # Sort results: successful runs first (sorted by time), then unrun targets
    sorted_items = sorted(
        [item for item in results.items() if item[1] is not None],
        key=lambda x: x[1]
    )
    for name, elapsed in results.items():
        if elapsed is None:
            sorted_items.append((name, None))

    table = Table(title="Binary-Trees Benchmarks", show_header=True, header_style="bold magenta")
    table.add_column("Language", style="bold white", width=12)
    table.add_column("Time (s)", justify="right")
    table.add_column("Relative to C", justify="right")
    if args.historic:
        table.add_column("vs Last Run", justify="right")
    table.add_column("Status", justify="center")

    for name, elapsed in sorted_items:
        if elapsed is None:
            if args.historic:
                table.add_row(name, "-", "-", "-", "[red]Not Run[/red]")
            else:
                table.add_row(name, "-", "-", "[red]Not Run[/red]")
        else:
            if name == "C":
                rel_str = "[bold green]1.00x (baseline)[/bold green]"
            elif c_time is not None:
                ratio = elapsed / c_time
                if ratio < 3.0:
                    rel_str = f"[green]{ratio:.2f}x[/green]"
                elif ratio < 10.0:
                    rel_str = f"[yellow]{ratio:.2f}x[/yellow]"
                else:
                    rel_str = f"[red]{ratio:.2f}x[/red]"
            else:
                rel_str = "N/A"
            
            if args.historic:
                old_time = latest_cache.get(name)
                if old_time is not None:
                    diff = elapsed - old_time
                    pct = (diff / old_time) * 100
                    if pct > 0:
                        vs_last = f"[red]+{pct:.1f}%[/red]"
                    elif pct < 0:
                        vs_last = f"[green]{pct:.1f}%[/green]"
                    else:
                        vs_last = "[white]0.0%[/white]"
                else:
                    vs_last = "-"
                table.add_row(name, f"{elapsed:.4f}s", rel_str, vs_last, "[green]OK[/green]")
            else:
                table.add_row(name, f"{elapsed:.4f}s", rel_str, "[green]OK[/green]")

    console.print("\n")
    console.print(table)

    # Save cache
    new_run_results = {}
    for name, elapsed in results.items():
        if elapsed is not None:
            new_run_results[name] = elapsed
            
    if new_run_results:
        import datetime
        timestamp = datetime.datetime.now().isoformat(timespec='seconds')
        if "runs" not in cache:
            cache["runs"] = []
        cache["runs"].append({
            "timestamp": timestamp,
            "results": new_run_results
        })
        try:
            with open(cache_file, "w") as f:
                json.dump(cache, f, indent=2)
        except Exception as e:
            console.print(f"[red]Failed to save cache: {e}[/red]")

    # Build artifacts are preserved after execution as requested.

if __name__ == "__main__":
    main()
