#!/usr/bin/env python3
"""Compare the run time of each benchmark as built by gaiarusted,
by rustc without optimisation, and by rustc -O, and the time each
compiler takes to build it.

usage: benchmarks/run.py [--runs N] [name ...]      (from the gaiarusted directory)
"""
import os, re, subprocess, sys, time

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
GAIA = os.path.join(ROOT, "target", "release", "gaiarusted")
WORK = os.path.join(ROOT, "target", "benchmarks")


def build(command):
    """Build; returns the time taken, or the first error message on failure."""
    start = time.perf_counter()
    result = subprocess.run(command, capture_output=True, cwd=ROOT)
    elapsed = time.perf_counter() - start
    if result.returncode == 0:
        return elapsed, None
    lines = [re.sub(r"\x1b\[[0-9;]*m", "", line).strip() for line in result.stderr.decode(errors="replace").splitlines()]
    errors = [line.split(":", 1)[1].strip() for line in lines if line.startswith("error") and "compilation failed" not in line]
    return None, (errors[0] if errors else "build failed")


def run_once(exe):
    """Wall time of one run, with the program's output (None if it failed)."""
    start = time.perf_counter()
    result = subprocess.run([exe], capture_output=True)
    elapsed = time.perf_counter() - start
    return (elapsed, result.stdout) if result.returncode == 0 else (None, None)


def best_times(exes, runs):
    """The fastest of `runs` runs of each executable. Runs are interleaved,
    so that load from elsewhere on the machine falls on all of them alike."""
    best, outputs = {}, {}
    for _ in range(runs):
        for kind, exe in exes.items():
            if kind in outputs and outputs[kind] is None:
                continue
            elapsed, output = run_once(exe)
            outputs[kind] = output
            if elapsed is not None:
                best[kind] = min(best.get(kind, elapsed), elapsed)
    return {kind: best.get(kind) for kind in exes}, outputs


def main():
    os.makedirs(WORK, exist_ok=True)
    subprocess.run(["cargo", "build", "--release", "--quiet"], cwd=ROOT, check=True, capture_output=True)
    args = sys.argv[1:]
    runs = 5
    if "--runs" in args:
        at = args.index("--runs")
        runs = int(args[at + 1])
        del args[at:at + 2]
    names = args or sorted(f[:-3] for f in os.listdir(HERE) if f.endswith(".rs"))
    print(f"{'benchmark':<14}{'gaiarusted':>12}{'rustc -O0':>12}{'rustc -O':>12}{'vs -O0':>9}{'vs -O':>8}"
          f"{'build gaia':>12}{'build rustc -O':>16}")
    failures = []
    for name in names:
        source = os.path.join(HERE, name + ".rs")
        builds = {
            "gaia": [GAIA, source, "-o", os.path.join(WORK, name + ".gaia")],
            "debug": ["rustc", "--edition", "2021", "-A", "warnings", "-C", "opt-level=0", "-o", os.path.join(WORK, name + ".debug"), source],
            "release": ["rustc", "--edition", "2021", "-A", "warnings", "-O", "-o", os.path.join(WORK, name + ".release"), source],
        }
        built = {kind: build(command) for kind, command in builds.items()}
        exes = {kind: command[command.index("-o") + 1] for kind, command in builds.items() if built[kind][0] is not None}
        if built["gaia"][1]:
            failures.append((name, built["gaia"][1]))
        times, outputs = best_times(exes, runs)
        for kind in builds:
            times.setdefault(kind, None)
            outputs.setdefault(kind, None)

        def show(seconds):
            return f"{seconds * 1000:9.0f} ms" if seconds is not None else "      failed"

        def ratio(kind):
            if times["gaia"] is None or times[kind] is None:
                return "      -"
            return f"{times['gaia'] / times[kind]:6.2f}x"

        wrong = "" if outputs["gaia"] is None or outputs["gaia"] == outputs["release"] else "   OUTPUT DIFFERS"
        build_times = show(built["gaia"][0]) + "    " + show(built["release"][0])
        print(f"{name:<14}{show(times['gaia'])}{show(times['debug'])}{show(times['release'])}{ratio('debug'):>9}{ratio('release'):>8}{build_times}{wrong}")
    if failures:
        print("\ngaiarusted cannot build:")
        for name, reason in failures:
            print(f"  {name:<14} {reason}")


if __name__ == "__main__":
    main()
