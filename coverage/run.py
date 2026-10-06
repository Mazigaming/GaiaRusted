#!/usr/bin/env python3
"""Feature coverage: what of Rust does gaiarusted compile correctly?

Every program in this directory exercises one area of the language or its
standard library. Each is built by rustc (the reference) and by gaiarusted at
-O0 and -O2, run (with `<name>.stdin` as input when present), and its output
and exit code compared with rustc's build.

usage: coverage/run.py [--harvest] [name_filter ...]
       (from the gaiarusted directory; results also go to target/coverage/)

--harvest lists every error in each program that does not compile, not
just the first: the line an error points at is removed and the program
compiled again, as long as that line is a statement on its own.

Unlike conformance/, failures here are expected: this measures how much of
Rust is covered, and lists what is missing.
"""
import json, os, re, subprocess, sys
from collections import Counter, defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
GAIA = os.path.join(ROOT, "target", "release", "gaiarusted")
WORK = os.path.join(ROOT, "target", "coverage")
LEVELS = ["0", "2"]
ANSI = re.compile(r"\x1b\[[0-9;]*m")


def run(command, stdin=None, timeout=20):
    try:
        done = subprocess.run(command, input=stdin, capture_output=True, timeout=timeout)
        return done.returncode, done.stdout.decode(errors="replace"), done.stderr.decode(errors="replace")
    except subprocess.TimeoutExpired:
        return None, "", "timed out"


def first_error(stderr):
    """The compiler's first error message, without colours and source excerpt."""
    lines = [ANSI.sub("", line).strip() for line in stderr.splitlines()]
    for index, line in enumerate(lines):
        if line.startswith("error") and "compilation failed" not in line:
            message = line.split(":", 1)[1].strip() if ":" in line else line
            if not message and index + 1 < len(lines):
                message = lines[index + 1]
            return message
    meaningful = [line for line in lines if line and not line.startswith("Compiling")]
    return meaningful[0] if meaningful else "unknown failure"


LOCATION = re.compile(r"-->\s+(\S+?):(\d+):\d+")
ITEM = re.compile(r"^(pub\s+)?(fn|impl|struct|enum|trait|type|const|static|mod|unsafe fn|#\[)\b")


def removable(line):
    """A line that is a complete statement or expression on its own."""
    text = line.strip()
    balanced = all(text.count(a) == text.count(b) for a, b in ["()", "[]", "{}"])
    return bool(text) and balanced and not ITEM.match(text) and text not in ("}", "{")


def harvest(name, source):
    """All the errors in a program, found by removing what each points at."""
    lines = open(source).read().split("\n")
    copy = os.path.join(WORK, f"{name}.rs")
    exe = os.path.join(WORK, f"{name}.harvest")
    errors = []
    for _ in range(40):
        with open(copy, "w") as out:
            out.write("\n".join(lines))
        code, _, err = run([GAIA, copy, "-o", exe, "-O0"], timeout=120)
        if code == 0:
            return errors, True
        message = first_error(err)
        if message not in errors:
            errors.append(message)
        location = LOCATION.search(ANSI.sub("", err))
        if not location or os.path.abspath(location.group(1)) != os.path.abspath(copy):
            break
        line = int(location.group(2)) - 1
        if not removable(lines[line]):
            break
        lines[line] = ""
    return errors, False


def first_difference(expected, actual):
    e, a = expected.splitlines(), actual.splitlines()
    for index, (x, y) in enumerate(zip(e, a)):
        if x != y:
            return f"line {index + 1}: expected {x!r}, got {y!r}"
    if len(e) != len(a):
        return f"expected {len(e)} lines, got {len(a)}"
    return "trailing whitespace differs"


def check(name, source, stdin, expected, level):
    exe = os.path.join(WORK, f"{name}.gaia{level}")
    code, _, err = run([GAIA, source, "-o", exe, f"-O{level}"], timeout=120)
    if code != 0:
        return {"status": "compile", "detail": first_error(err)}
    code, out, err = run([exe], stdin)
    if code is None:
        return {"status": "timeout", "detail": "ran over 20 s"}
    if code < 0:
        return {"status": "crash", "detail": f"killed by signal {-code}"}
    if out != expected["stdout"]:
        return {"status": "wrong", "detail": first_difference(expected["stdout"], out)}
    if code != expected["code"]:
        return {"status": "wrong", "detail": f"exit code {code}, expected {expected['code']}"}
    return {"status": "pass", "detail": ""}


def main():
    args = sys.argv[1:]
    do_harvest = "--harvest" in args
    args = [a for a in args if a != "--harvest"]
    os.makedirs(WORK, exist_ok=True)
    subprocess.run(["cargo", "build", "--release", "--quiet", "-j", "4"], cwd=ROOT, check=True)

    names = sorted(f[:-3] for f in os.listdir(HERE) if f.endswith(".rs"))
    names = [n for n in names if not args or any(a in n for a in args)]
    results = {}
    for name in names:
        source = os.path.join(HERE, name + ".rs")
        stdin_file = os.path.join(HERE, name + ".stdin")
        stdin = open(stdin_file, "rb").read() if os.path.exists(stdin_file) else b""
        reference = os.path.join(WORK, f"{name}.rustc")
        code, _, err = run(["rustc", "--edition", "2021", "-A", "warnings", "-O", "-o", reference, source], timeout=120)
        if code != 0:
            results[name] = {"area": name.split("_")[0], "levels": {}, "bad_test": first_error(err)}
            print(f"{name:<36} BAD TEST: rustc rejects it: {first_error(err)}")
            continue
        code, out, _ = run([reference], stdin)
        expected = {"stdout": out, "code": code}
        levels = {level: check(name, source, stdin, expected, level) for level in LEVELS}
        results[name] = {"area": name.split("_")[0], "levels": levels}
        o0, o2 = levels["0"], levels["2"]
        line = f"{name:<36} {o2['status'].upper():<8}"
        if o2["status"] != "pass":
            line += f" {o2['detail']}"
        if o0["status"] != o2["status"]:
            line += f"   [-O0: {o0['status']}{': ' + o0['detail'] if o0['detail'] else ''}]"
        print(line, flush=True)

    # Summary by area, at -O2.
    by_area = defaultdict(Counter)
    for result in results.values():
        status = result["levels"].get("2", {}).get("status", "bad test")
        by_area[result["area"]][status] += 1
    print(f"\n{'area':<12}{'pass':>6}{'total':>7}")
    totals = Counter()
    for area in sorted(by_area):
        counts = by_area[area]
        totals.update(counts)
        print(f"{area:<12}{counts['pass']:>6}{sum(counts.values()):>7}")
    print(f"{'all':<12}{totals['pass']:>6}{sum(totals.values()):>7}   " + ", ".join(f"{k} {v}" for k, v in sorted(totals.items())))

    optimiser_bugs = [n for n, r in results.items() if r["levels"].get("0", {}).get("status") == "pass" and r["levels"].get("2", {}).get("status") != "pass"]
    if optimiser_bugs:
        print("\npasses at -O0 but not at -O2 (optimiser bugs):", ", ".join(optimiser_bugs))

    if do_harvest:
        print("\nEverything that stops each failing program from compiling:")
        for name, result in results.items():
            if result["levels"].get("2", {}).get("status") != "compile":
                continue
            errors, compiles = harvest(name, os.path.join(HERE, name + ".rs"))
            result["missing"] = errors
            suffix = "" if compiles else "  (+ possibly more after this)"
            print(f"\n{name}{suffix}")
            for error in errors:
                print(f"    - {error}")

    with open(os.path.join(WORK, "results.json"), "w") as out:
        json.dump(results, out, indent=1)


if __name__ == "__main__":
    main()
