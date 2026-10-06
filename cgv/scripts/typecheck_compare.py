#!/usr/bin/env python3
# Run mypy and pyright, both strict, on the pre and fix trees of every
# directory-based case in a bench corpus, and print one Markdown row per case
# next to CGV's outcome from a bench result. A type checker "flags" a case when
# the pre tree has an error in a bug's file that the fix tree does not have
# (same file and rule; lines and message wording may move), so noise the fix
# leaves in place does not count.
#
# Usage: scripts/typecheck_compare.py [--corpus DIR] [--bench-result JSON]
#                                     [--python PATH]
# --python names an interpreter with the pinned packages of
# bench/typecheckers/requirements.txt installed. Exit 0 ok, 2 harness failure
# (ValueError covers invalid TOML, invalid JSON and non-UTF-8 input; OSError
# covers an unreadable tree; a command that cannot start is a HarnessError
# naming the tool).
import argparse
import collections
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_CORPUS = REPO_ROOT / "bench" / "corpus"
DEFAULT_BENCH_RESULT = REPO_ROOT / "bench" / "baseline.json"
SETTINGS_MODULE = "cgv_typecheck_settings"
MYPY_LINE = re.compile(r"^(?P<file>[^:]+\.py):(?P<line>\d+): error: (?P<msg>.*?)(?:  \[(?P<code>[\w-]+)\])?$")


class HarnessError(Exception):
    pass


def run_tool(tool, cmd, **kwargs):
    """subprocess.run, with a failure to start the command named after the tool."""
    try:
        return subprocess.run(cmd, **kwargs)
    except OSError as e:
        raise HarnessError(f"cannot run {tool} with {cmd[0]}: {e}")


def write_config(tree, python):
    apps = sorted(p.parent.name for p in tree.glob("*/models.py"))
    (tree / f"{SETTINGS_MODULE}.py").write_text(
        f'SECRET_KEY = "x"\nINSTALLED_APPS = {apps!r}\n'
        'DEFAULT_AUTO_FIELD = "django.db.models.AutoField"\n'
    )
    (tree / "mypy.ini").write_text(
        "[mypy]\nstrict = True\n"
        "plugins = mypy_django_plugin.main, pydantic.mypy\n"
        f"exclude = {SETTINGS_MODULE}\\.py\n\n"
        f"[mypy.plugins.django-stubs]\ndjango_settings_module = {SETTINGS_MODULE}\n"
    )
    (tree / "pyrightconfig.json").write_text(json.dumps({
        "typeCheckingMode": "strict",
        "exclude": [f"{SETTINGS_MODULE}.py"],
    }))


def run_mypy(tree, python):
    proc = run_tool(
        "mypy", [python, "-m", "mypy", "--config-file", "mypy.ini", "--no-error-summary",
         "--no-pretty", "--hide-error-context", "--cache-dir", "/dev/null", "."],
        cwd=tree, capture_output=True, text=True,
        env={**os.environ, "MYPY_FORCE_COLOR": "0", "NO_COLOR": "1", "TERM": "dumb"},
    )
    if proc.returncode not in (0, 1):
        raise HarnessError(f"mypy failed in {tree}:\n{proc.stdout}{proc.stderr}")
    diags = []
    for line in proc.stdout.splitlines():
        m = MYPY_LINE.match(line)
        if m:
            diags.append((Path(m["file"]).as_posix(), int(m["line"]), m["code"] or "", m["msg"]))
    return diags


def run_pyright(tree, python):
    proc = run_tool(
        "pyright", [python, "-m", "pyright", "--outputjson", "--pythonpath", python, "-p", "pyrightconfig.json"],
        cwd=tree, capture_output=True, text=True,
    )
    try:
        out = json.loads(proc.stdout)
    except json.JSONDecodeError:
        raise HarnessError(f"pyright failed in {tree}:\n{proc.stdout}{proc.stderr}")
    diags = []
    try:
        for d in out["generalDiagnostics"]:
            if d["severity"] != "error":
                continue
            rel = Path(d["file"]).resolve().relative_to(tree.resolve()).as_posix()
            diags.append((rel, d["range"]["start"]["line"] + 1, d.get("rule", ""), d["message"]))
    except (KeyError, TypeError, ValueError) as e:
        raise HarnessError(f"pyright output in {tree} has an unexpected shape: {e!r}")
    return diags


def check_tree(src, python, work):
    tree = work / src.name
    shutil.copytree(src, tree)
    write_config(tree, python)
    return {"mypy": run_mypy(tree, python), "pyright": run_pyright(tree, python)}


def versions(python):
    out = {}
    for pkg in ("mypy", "pyright", "django-stubs", "django", "pydantic"):
        proc = run_tool(
            pkg, [python, "-c", f"import importlib.metadata as m; print(m.version({pkg!r}))"],
            capture_output=True, text=True,
        )
        if proc.returncode != 0:
            raise HarnessError(f"{pkg} is not installed for {python}")
        out[pkg] = proc.stdout.strip()
    return out


def load_cases(corpus):
    cases = []
    for case_toml in sorted((corpus / "cases").glob("*/case.toml")):
        try:
            with open(case_toml, "rb") as f:
                case = tomllib.load(f)
            case_dir = case_toml.parent
            if "pre" not in case:
                continue
            case_id = case.get("id", case_dir.name)
            pre, fix = case_dir / case["pre"], case_dir / case["fix"]
            files = sorted({b["file"] for b in case["bug"]})
            if not isinstance(case_id, str) or not case_id:
                raise TypeError(f"id must be a non-empty string, not {case_id!r}")
            if not isinstance(case["kind"], str):
                raise TypeError(f"kind must be a string, not {case['kind']!r}")
            if not isinstance(case["in_scope"], bool):
                raise TypeError(f"in_scope must be true or false, not {case['in_scope']!r}")
            if not files:
                raise ValueError("bug must name at least one file")
            if not all(isinstance(f, str) and f for f in files):
                raise TypeError(f"bug files must be non-empty strings, not {files!r}")
            for tree in (pre, fix):
                if not tree.is_dir():
                    raise OSError(f"{tree} is not a directory")
            cases.append({
                "id": case_id,
                "kind": case["kind"],
                "in_scope": case["in_scope"],
                "pre": pre,
                "fix": fix,
                "files": files,
            })
        except (KeyError, TypeError, OSError, ValueError, RecursionError) as e:
            raise HarnessError(f"{case_toml} is malformed: {e!r}")
    return cases


def cleared(pre, fix, files):
    """Errors in a bug's file on the pre tree that the fix tree does not have."""
    fix_keys = collections.Counter((f, code) for f, _, code, _ in fix if f in files)
    out = []
    for f, line, code, _ in pre:
        if f in files:
            if fix_keys[(f, code)] > 0:
                fix_keys[(f, code)] -= 1
            else:
                out.append((f, line, code))
    return out


def cell(pre, fix, files):
    hits = cleared(pre, fix, files)
    if not hits:
        return "not flagged"
    return "flagged: " + ", ".join(f"line {line} `{code}`" for _, line, code in hits)


def display(path):
    path = path.resolve()
    return path.relative_to(REPO_ROOT) if path.is_relative_to(REPO_ROOT) else path


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument("--corpus", type=Path, default=DEFAULT_CORPUS)
    ap.add_argument("--bench-result", type=Path, default=DEFAULT_BENCH_RESULT)
    ap.add_argument("--python", default=sys.executable)
    args = ap.parse_args(argv)
    # The runners use cwd=<temp tree>, so a relative path to an interpreter
    # must be made absolute here. A bare command name is left to PATH lookup.
    if os.sep in args.python:
        args.python = os.path.abspath(args.python)
    try:
        vers = versions(args.python)
        try:
            cgv = {c["id"]: c["outcome"] for c in json.loads(args.bench_result.read_text())["cases"]}
        except (KeyError, TypeError, OSError, ValueError) as e:
            raise HarnessError(f"{args.bench_result} is malformed: {e!r}")
        rows = []
        for case in load_cases(args.corpus):
            with tempfile.TemporaryDirectory() as tmp:
                pre = check_tree(case["pre"], args.python, Path(tmp) / "pre")
                fix = check_tree(case["fix"], args.python, Path(tmp) / "fix")
            rows.append((case, pre, fix))
    except (HarnessError, OSError) as e:
        print(f"error: {e}", file=sys.stderr)
        return 2
    print(
        f"mypy {vers['mypy']} (`strict`, django-stubs {vers['django-stubs']}, pydantic plugin), "
        f"pyright {vers['pyright']} (`strict`), Django {vers['django']}, pydantic {vers['pydantic']}. "
        f"CGV outcome from `{display(args.bench_result)}`.\n"
    )
    print("| Case | Kind | CGV design reaches it | CGV | mypy | pyright |")
    print("|---|---|---|---|---|---|")
    for case, pre, fix in rows:
        print(
            f"| `{case['id']}` | {case['kind']} | {'yes' if case['in_scope'] else 'not yet reached'} "
            f"| {cgv.get(case['id'], 'n/a')} "
            f"| {cell(pre['mypy'], fix['mypy'], case['files'])} "
            f"| {cell(pre['pyright'], fix['pyright'], case['files'])} |"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
