#!/usr/bin/env python3
# Benchmark harness: replay known bugs through the full pipeline (Rust
# extractor + Lean checker) and report, per case, whether the pre-fix tree is
# flagged at the bug site and the fixed tree is cleared; plus precision over
# labelled findings. Corpus format and outcome classes: bench/README.md.
#
# Usage: scripts/bench.py run [--corpus DIR ...] [--cli PATH] [--checker PATH]
#                             [--out RESULT.json] [--compare PREV.json]
#                             [--format text|markdown] [--keep-work DIR]
# Exit: 0 ok, 1 regression under --compare, 2 bad corpus, missing binary,
# git failure or a pipeline run that failed (exit 2 or output not JSON).
# Requires: cargo build --release, cd prover && lake build, python3 >= 3.11.
import argparse
import io
import json
import os
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import tomllib
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_CORPUS = REPO_ROOT / "bench" / "corpus"
DEFAULT_CLI = REPO_ROOT / "target" / "release" / "crosscheck-contracts"
DEFAULT_CHECKER = REPO_ROOT / "prover" / ".lake" / "build" / "bin" / "contract-graph-checker"

KINDS = ("non-null", "length", "precision", "range", "choices", "type")
LABELS = ("true_bug", "benign", "false_positive")
IN_SCOPE_ORDER = ("CAUGHT", "DETECTED_NOT_CLEARED", "WARNING_ONLY", "MISSED")
OUT_SCOPE_ORDER = ("NOT_REPORTED", "SPURIOUS_ERROR")
OUTCOMES = IN_SCOPE_ORDER + OUT_SCOPE_ORDER + ("FAILED",)


class BenchError(Exception):
    """A harness failure: bad corpus file, missing binary, git failure."""


# ---------------------------------------------------------------- corpus


def _read_toml(path):
    try:
        with open(path, "rb") as f:
            return tomllib.load(f)
    except FileNotFoundError:
        raise BenchError(f"{path}: file not found")
    except tomllib.TOMLDecodeError as e:
        raise BenchError(f"{path}: invalid TOML: {e}")


def _str_list(value, where, key):
    if value is None:
        return []
    if not isinstance(value, list) or not all(isinstance(v, str) for v in value):
        raise BenchError(f"{where}: '{key}' must be a list of strings")
    return list(value)


def _expand_repo(value, base, where):
    if not isinstance(value, str):
        raise BenchError(f"{where}: 'repo' must be a string")
    expanded = os.path.expandvars(value)
    unset = re.search(r"\$\{?(\w+)", expanded)
    if unset:
        raise BenchError(f"{where}: environment variable {unset.group(1)} in repo '{value}' is not set")
    return (base / expanded).resolve()


def _check_id(value, where, what):
    if not isinstance(value, str) or not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]*", value):
        raise BenchError(f"{where}: {what} id {value!r} must be a name of letters, digits, '.', '_' or '-'")
    return value


def load_corpus(corpus_dir):
    corpus_dir = Path(corpus_dir).resolve()
    where = corpus_dir / "corpus.toml"
    meta = _read_toml(where)
    name = meta.get("name")
    if not isinstance(name, str) or not name:
        raise BenchError(f"{where}: missing 'name'")
    repo = _expand_repo(meta["repo"], corpus_dir, where) if "repo" in meta else None
    corpus = {
        "name": name,
        "dir": corpus_dir,
        "check_args": _str_list(meta.get("check_args"), where, "check_args"),
        "repo": repo,
        "cases": [],
        "labelled_runs": [],
        "labels": [],
    }
    for i, lr in enumerate(meta.get("labelled_run", [])):
        corpus["labelled_runs"].append(_load_labelled_run(lr, corpus, f"{where} [[labelled_run]] #{i + 1}"))
    cases_dir = corpus_dir / "cases"
    if cases_dir.is_dir():
        for case_dir in sorted(p for p in cases_dir.iterdir() if p.is_dir()):
            corpus["cases"].append(load_case(case_dir, corpus))
    labels_path = corpus_dir / "labels.jsonl"
    if labels_path.exists():
        corpus["labels"] = load_labels(labels_path)
    return corpus


def _load_tree(spec, corpus, where, dir_key, git_key, base):
    """A tree to check: a directory (dir_key) or a git revision (git_key)."""
    has_dir = dir_key in spec
    has_git = git_key in spec or "repo" in spec
    if has_dir and has_git:
        raise BenchError(f"{where}: give either '{dir_key}' (directory) or '{git_key}'/'repo' (git), not both")
    if has_dir:
        if not isinstance(spec[dir_key], str):
            raise BenchError(f"{where}: '{dir_key}' must be a string")
        path = (base / spec[dir_key]).resolve()
        if not path.is_dir():
            raise BenchError(f"{where}: '{dir_key}' directory {spec[dir_key]} not found")
        return {"dir": path}
    if git_key not in spec:
        raise BenchError(f"{where}: missing '{dir_key}' (directory) or '{git_key}' (git revision)")
    if not isinstance(spec[git_key], str):
        raise BenchError(f"{where}: '{git_key}' must be a string")
    repo = _expand_repo(spec["repo"], corpus["dir"], where) if "repo" in spec else corpus["repo"]
    if repo is None:
        raise BenchError(f"{where}: git revision given but no 'repo' here or in corpus.toml")
    return {"repo": repo, "rev": spec[git_key]}


def _load_labelled_run(lr, corpus, where):
    if "id" not in lr:
        raise BenchError(f"{where}: missing 'id'")
    return {
        "id": _check_id(lr["id"], where, "labelled run"),
        "tree": _load_tree(lr, corpus, where, "app", "rev", corpus["dir"]),
        "app_subdir": lr.get("app_subdir", ""),
        "check_args": _str_list(lr.get("check_args"), where, "check_args"),
    }


def load_case(case_dir, corpus):
    where = case_dir / "case.toml"
    spec = _read_toml(where)
    kind = spec.get("kind")
    if kind is None:
        raise BenchError(f"{where}: missing 'kind' (one of {', '.join(KINDS)})")
    if kind not in KINDS:
        raise BenchError(f"{where}: unknown kind '{kind}' (one of {', '.join(KINDS)})")
    if not isinstance(spec.get("in_scope"), bool):
        raise BenchError(f"{where}: missing 'in_scope' (true or false)")
    dir_keys = [k for k in ("pre", "fix") if k in spec]
    git_keys = [k for k in ("pre_rev", "fix_rev", "repo") if k in spec]
    if dir_keys and git_keys:
        raise BenchError(f"{where}: both directory keys ({', '.join(dir_keys)}) and git keys "
                         f"({', '.join(git_keys)}); a case is one or the other")
    pre_spec = {k: spec[k] for k in ("pre", "repo") if k in spec}
    fix_spec = {k: spec[k] for k in ("fix", "repo") if k in spec}
    if "pre_rev" in spec:
        pre_spec["rev"] = spec["pre_rev"]
    if "fix_rev" in spec:
        fix_spec["rev"] = spec["fix_rev"]
    if not dir_keys and not git_keys:
        raise BenchError(f"{where}: missing 'pre'/'fix' (directories) or 'pre_rev'/'fix_rev' (git revisions)")
    pre = _load_tree(pre_spec, corpus, where, "pre", "rev", case_dir)
    fix = _load_tree(fix_spec, corpus, where, "fix", "rev", case_dir)
    bugs = spec.get("bug")
    if not bugs:
        raise BenchError(f"{where}: missing [[bug]] (at least one bug site)")
    return {
        "id": _check_id(spec.get("id", case_dir.name), where, "case"),
        "description": spec.get("description", ""),
        "kind": kind,
        "in_scope": spec["in_scope"],
        "source": spec.get("source", "synthetic"),
        "pre": pre,
        "fix": fix,
        "app_subdir": spec.get("app_subdir", ""),
        "check_args": _str_list(spec.get("check_args"), where, "check_args"),
        "bugs": [_load_bug(b, f"{where} [[bug]] #{i + 1}") for i, b in enumerate(bugs)],
    }


def _load_bug(b, where):
    if not isinstance(b.get("file"), str):
        raise BenchError(f"{where}: missing 'file'")
    if ("line" in b) == ("lines" in b):
        raise BenchError(f"{where}: give exactly one of 'line' or 'lines'")
    if "line" in b:
        if not isinstance(b["line"], int):
            raise BenchError(f"{where}: 'line' must be an integer")
        lo = hi = b["line"]
    else:
        lines = b["lines"]
        if not (isinstance(lines, list) and len(lines) == 2 and all(isinstance(x, int) for x in lines)):
            raise BenchError(f"{where}: 'lines' must be [first, last]")
        lo, hi = lines
        if lo > hi:
            raise BenchError(f"{where}: 'lines' must be [first, last] with first <= last")
    target = b.get("target")
    if target is not None and not isinstance(target, str):
        raise BenchError(f"{where}: 'target' must be a string")
    return {"file": b["file"], "lo": lo, "hi": hi, "target": target}


def load_labels(path):
    labels = []
    for n, line in enumerate(Path(path).read_text().splitlines(), 1):
        if not line.strip():
            continue
        where = f"{path}:{n}"
        try:
            obj = json.loads(line)
        except json.JSONDecodeError as e:
            raise BenchError(f"{where}: invalid JSON: {e}")
        for key in ("run", "source", "target", "hop", "kind", "label"):
            if key not in obj:
                raise BenchError(f"{where}: missing '{key}'")
        if obj["label"] not in LABELS:
            raise BenchError(f"{where}: unknown label '{obj['label']}' (one of {', '.join(LABELS)})")
        if not isinstance(obj["hop"], list) or not all(isinstance(h, str) for h in obj["hop"]):
            raise BenchError(f"{where}: 'hop' must be a list of strings")
        obj["identity"] = (obj["source"], obj["target"], "|".join(obj["hop"]), obj["kind"], obj.get("site_file"))
        if any(o["run"] == obj["run"] and o["identity"] == obj["identity"] for o in labels):
            raise BenchError(f"{where}: duplicate label for run '{obj['run']}' with the same identity")
        labels.append(obj)
    return labels


# ---------------------------------------------------------------- running


def make_runner(cli, checker, workdir):
    """runner(app_dir, args) -> (exit_code, parsed JSON or None)."""
    counter = [0]

    def run(app_dir, args):
        counter[0] += 1
        db = Path(workdir) / f"run{counter[0]:03d}.sqlite"
        cmd = [str(cli), "contracts", "check", str(app_dir), "--lean-checker", str(checker),
               "--output-db", str(db), *args]
        proc = subprocess.run(cmd, capture_output=True, text=True)
        try:
            data = json.loads(proc.stdout)
        except json.JSONDecodeError:
            data = None
        if proc.returncode not in (0, 1) or data is None:
            tail = "\n".join(proc.stderr.strip().splitlines()[-5:])
            print(f"bench: pipeline failed on {app_dir} (exit {proc.returncode})\n{tail}", file=sys.stderr)
        return proc.returncode, data

    return run


def prepare_tree(tree, app_subdir, dest):
    """The app root to check: a directory as is, or a git revision archived into dest."""
    if "dir" in tree:
        return tree["dir"] / app_subdir if app_subdir else tree["dir"]
    cmd = ["git", "-C", str(tree["repo"]), "archive", "--format=tar", tree["rev"]]
    if app_subdir:
        cmd += ["--", app_subdir]
    try:
        proc = subprocess.run(cmd, capture_output=True)
    except OSError as e:
        raise BenchError(f"git archive {tree['rev']} in {tree['repo']} failed: {e}")
    if proc.returncode != 0:
        raise BenchError(f"git archive {tree['rev']} in {tree['repo']} failed: "
                         f"{proc.stderr.decode(errors='replace').strip()}")
    dest = Path(dest)
    shutil.rmtree(dest, ignore_errors=True)
    dest.mkdir(parents=True)
    try:
        with tarfile.open(fileobj=io.BytesIO(proc.stdout)) as tar:
            if hasattr(tarfile, "data_filter"):
                tar.extractall(dest, filter="data")
            else:
                tar.extractall(dest)
    except (tarfile.TarError, OSError) as e:
        raise BenchError(f"extracting {tree['rev']} of {tree['repo']} failed: {e}")
    return dest / app_subdir if app_subdir else dest


def _check(runner, tree, app_subdir, dest, args):
    """(exit, data, failed)."""
    try:
        root = prepare_tree(tree, app_subdir, dest)
    except BenchError as e:
        print(f"bench: {e}", file=sys.stderr)
        return None, None, True
    code, data = runner(root, args)
    failed = code not in (0, 1) or not isinstance(data, dict) or not isinstance(data.get("results"), list)
    return code, data, failed


# ---------------------------------------------------------------- matching


def kind_of(r):
    req = r.get("target_requirement") or ""
    return req.split()[0] if req.split() else ""


def is_error(r):
    return r.get("severity") == "error" and r.get("status") != "incomplete"


def is_warning(r):
    return r.get("severity") != "error" and r.get("status") != "incomplete"


def identity(r):
    site = r.get("site") or {}
    return ((r.get("source") or {}).get("name"), (r.get("target") or {}).get("name"),
            "|".join(r.get("hop") or []), kind_of(r), site.get("file"))


def sort_key(r):
    site = r.get("site") or {}
    return (site.get("file") or "", site.get("line") or 0,
            (r.get("target") or {}).get("name") or "", (r.get("source") or {}).get("name") or "")


def _same_file(a, b):
    return a == b or a.endswith("/" + b) or b.endswith("/" + a)


def matches(r, bug, kind):
    site = r.get("site")
    if kind_of(r) != kind or not site or site.get("file") is None or site.get("line") is None:
        return False
    if not _same_file(site["file"], bug["file"]) or not bug["lo"] <= site["line"] <= bug["hi"]:
        return False
    return bug["target"] is None or bug["target"] in ((r.get("target") or {}).get("name") or "")


def _rank(sorted_results, matched):
    for i, r in enumerate(sorted_results, 1):
        if any(r is m for m in matched):
            return [i, len(sorted_results)]
    return None


def classify(case, pre_results, fix_results):
    """(outcome, rank, matched pre results) for one case's two runs."""
    def hit(r):
        return any(matches(r, b, case["kind"]) for b in case["bugs"])
    pre_errors = sorted((r for r in pre_results if is_error(r)), key=sort_key)
    pre_warnings = sorted((r for r in pre_results if is_warning(r)), key=sort_key)
    m_err = [r for r in pre_errors if hit(r)]
    m_warn = [r for r in pre_warnings if hit(r)]
    rank = None
    if case["in_scope"]:
        if m_err:
            ids = {identity(r) for r in m_err}
            cleared = not any(is_error(r) and identity(r) in ids for r in fix_results)
            outcome = "CAUGHT" if cleared else "DETECTED_NOT_CLEARED"
            rank = _rank(pre_errors, m_err)
        elif m_warn:
            outcome = "WARNING_ONLY"
            rank = _rank(pre_warnings, m_warn)
        else:
            outcome = "MISSED"
    elif m_err:
        outcome = "SPURIOUS_ERROR"
        rank = _rank(pre_errors, m_err)
    else:
        outcome = "NOT_REPORTED"
    return outcome, rank, m_err + m_warn


def _brief(r):
    site = r.get("site") or {}
    return {"severity": r.get("severity"), "source": (r.get("source") or {}).get("name"),
            "target": (r.get("target") or {}).get("name"),
            "site": f"{site.get('file')}:{site.get('line')}" if site else None}


def _counts(code, data, failed):
    if failed:
        return {"exit": code, "errors": None, "warnings": None}
    rs = data["results"]
    return {"exit": code, "errors": sum(map(is_error, rs)), "warnings": sum(map(is_warning, rs))}


def run_case(case, corpus, runner, workdir):
    args = corpus["check_args"] + case["check_args"]
    runs = {}
    for side in ("pre", "fix"):
        dest = Path(workdir) / f"case-{case['id']}-{side}"
        runs[side] = _check(runner, case[side], case["app_subdir"], dest, args)
    failed = runs["pre"][2] or runs["fix"][2]
    if failed:
        outcome, rank, matched = "FAILED", None, []
    else:
        outcome, rank, matched = classify(case, runs["pre"][1]["results"], runs["fix"][1]["results"])
    return {"id": case["id"], "corpus": corpus["name"], "kind": case["kind"], "in_scope": case["in_scope"],
            "source": case["source"], "outcome": outcome, "rank": rank,
            "pre": _counts(*runs["pre"]), "fix": _counts(*runs["fix"]),
            "matched": [_brief(r) for r in matched]}


def join_labels(run_id, corpus_name, errors, labels):
    """Precision of a labelled run: its errors joined with labels by identity."""
    by_id = {}
    for lab in labels:
        by_id[lab["identity"]] = lab
    counts = {k: 0 for k in LABELS}
    causes = {}
    unlabelled = 0
    current = set()
    for r in errors:
        ident = identity(r)
        current.add(ident)
        lab = by_id.get(ident)
        if lab is None:
            unlabelled += 1
            continue
        counts[lab["label"]] += 1
        if lab["label"] == "false_positive":
            cause = lab.get("cause") or "unspecified"
            causes[cause] = causes.get(cause, 0) + 1
    labelled = sum(counts.values())
    return {"id": run_id, "corpus": corpus_name, "errors": len(errors), "labelled": labelled, **counts,
            "precision": round(counts["true_bug"] / labelled, 4) if labelled else None,
            "unlabelled": unlabelled, "stale_labels": sum(1 for k in by_id if k not in current),
            "false_positive_causes": dict(sorted(causes.items()))}


def run_labelled(lr, corpus, runner, workdir):
    labels = [lab for lab in corpus["labels"] if lab["run"] == lr["id"]]
    dest = Path(workdir) / f"run-{lr['id']}"
    code, data, failed = _check(runner, lr["tree"], lr["app_subdir"], dest,
                                corpus["check_args"] + lr["check_args"])
    if failed:
        return {"id": lr["id"], "corpus": corpus["name"], "failed": True}
    return join_labels(lr["id"], corpus["name"], [r for r in data["results"] if is_error(r)], labels)


def summarize(cases):
    s = {"in_scope": sum(c["in_scope"] for c in cases)}
    for o in IN_SCOPE_ORDER:
        s[o] = sum(c["outcome"] == o for c in cases)
    s["out_of_scope"] = sum(not c["in_scope"] for c in cases)
    for o in OUT_SCOPE_ORDER + ("FAILED",):
        s[o] = sum(c["outcome"] == o for c in cases)
    return s


def tool_commit():
    try:
        proc = subprocess.run(["git", "-C", str(REPO_ROOT), "rev-parse", "HEAD"], capture_output=True, text=True)
    except OSError:
        return None
    return proc.stdout.strip() if proc.returncode == 0 else None


def run_bench(corpora, runner, workdir):
    seen = {}
    seen_runs = {}
    for corpus in corpora:
        for lr in corpus["labelled_runs"]:
            if lr["id"] in seen_runs:
                raise BenchError(f"duplicate labelled run id '{lr['id']}' in corpora "
                                 f"{seen_runs[lr['id']]} and {corpus['name']}")
            seen_runs[lr["id"]] = corpus["name"]
        for case in corpus["cases"]:
            if case["id"] in seen:
                raise BenchError(f"duplicate case id '{case['id']}' in corpora {seen[case['id']]} and {corpus['name']}")
            seen[case["id"]] = corpus["name"]
        known = {lr["id"] for lr in corpus["labelled_runs"]}
        for lab in corpus["labels"]:
            if lab["run"] not in known:
                print(f"bench: {corpus['name']}: label for unknown run '{lab['run']}' ignored", file=sys.stderr)
    cases = [run_case(c, corpus, runner, workdir) for corpus in corpora for c in corpus["cases"]]
    labelled = [run_labelled(lr, corpus, runner, workdir) for corpus in corpora for lr in corpus["labelled_runs"]]
    return {"schema": 1, "tool_commit": tool_commit(), "corpora": [c["name"] for c in corpora],
            "cases": cases, "labelled_runs": labelled, "summary": summarize(cases)}


# ---------------------------------------------------------------- compare


def _score(case):
    """(scope, badness); FAILED is worst in either scope."""
    o = case["outcome"]
    if o == "FAILED":
        return len(OUTCOMES)
    return (IN_SCOPE_ORDER if case["in_scope"] else OUT_SCOPE_ORDER).index(o)


def compare(prev, cur):
    """(report lines, regression found)."""
    lines = []
    regression = False
    prev_cases = {c["id"]: c for c in prev.get("cases", [])}
    cur_cases = {c["id"]: c for c in cur["cases"]}
    for cid, c in cur_cases.items():
        p = prev_cases.get(cid)
        if p is None:
            lines.append(f"new case: {cid} {c['outcome']}")
            continue
        if p["outcome"] != c["outcome"]:
            comparable = p["in_scope"] == c["in_scope"] or "FAILED" in (p["outcome"], c["outcome"])
            if not comparable:
                lines.append(f"scope changed: {cid} {p['outcome']} -> {c['outcome']}")
            elif _score(c) > _score(p):
                regression = True
                lines.append(f"REGRESSION: {cid} {p['outcome']} -> {c['outcome']}")
            else:
                lines.append(f"improvement: {cid} {p['outcome']} -> {c['outcome']}")
        elif p.get("rank") != c.get("rank"):
            lines.append(f"rank changed: {cid} {_fmt_rank(p.get('rank'))} -> {_fmt_rank(c.get('rank'))}")
    for cid in prev_cases:
        if cid not in cur_cases:
            lines.append(f"removed case: {cid} (was {prev_cases[cid]['outcome']})")
    prev_runs = {r["id"]: r for r in prev.get("labelled_runs", [])}
    cur_runs = {r["id"]: r for r in cur["labelled_runs"]}
    for rid, r in cur_runs.items():
        p = prev_runs.get(rid)
        if p is None:
            lines.append(f"new labelled run: {rid}")
            continue
        for key in ("precision", "false_positive", "stale_labels"):
            if p.get(key) != r.get(key):
                lines.append(f"labelled run {rid}: {key} {p.get(key)} -> {r.get(key)}")
    for rid in prev_runs:
        if rid not in cur_runs:
            lines.append(f"removed labelled run: {rid}")
    return lines, regression


# ---------------------------------------------------------------- output


def _fmt_rank(rank):
    return f"{rank[0]} of {rank[1]}" if rank else "-"


def _fmt_ew(side):
    return "-" if side["errors"] is None else f"{side['errors']}/{side['warnings']}"


def _table(header, rows, fmt):
    if fmt == "markdown":
        out = ["| " + " | ".join(header) + " |", "|" + "---|" * len(header)]
        out += ["| " + " | ".join(str(c) for c in row) + " |" for row in rows]
        return out
    widths = [max(len(str(x)) for x in col) for col in zip(header, *rows)]
    return ["  ".join(str(c).ljust(w) for c, w in zip(row, widths)).rstrip() for row in [header, *rows]]


def render(result, fmt):
    rows = [(c["id"], c["kind"], "in" if c["in_scope"] else "out", c["outcome"], _fmt_rank(c["rank"]),
             _fmt_ew(c["pre"]), _fmt_ew(c["fix"])) for c in result["cases"]]
    out = _table(("case", "kind", "scope", "outcome", "rank", "pre e/w", "fix e/w"), rows, fmt)
    s = result["summary"]
    out += ["", f"in scope {s['in_scope']}: " + ", ".join(f"{o} {s[o]}" for o in IN_SCOPE_ORDER)
            + f"; out of scope {s['out_of_scope']}: " + ", ".join(f"{o} {s[o]}" for o in OUT_SCOPE_ORDER)
            + f"; FAILED {s['FAILED']}"]
    if result["labelled_runs"]:
        lrows = []
        for r in result["labelled_runs"]:
            if r.get("failed"):
                lrows.append((r["id"], "FAILED", "", "", "", "", "", "", ""))
                continue
            prec = "-" if r["precision"] is None else f"{r['precision']:.2f}"
            lrows.append((r["id"], r["errors"], r["labelled"], r["true_bug"], r["benign"],
                          r["false_positive"], prec, r["unlabelled"], r["stale_labels"]))
        out += [""] + _table(("labelled run", "errors", "labelled", "true_bug", "benign", "false_positive",
                              "precision", "unlabelled", "stale"), lrows, fmt)
        for r in result["labelled_runs"]:
            if not r.get("failed") and r["false_positive_causes"]:
                out.append(f"{r['id']} false positive causes: "
                           + ", ".join(f"{k} {v}" for k, v in r["false_positive_causes"].items()))
    return "\n".join(out)


# ---------------------------------------------------------------- main


def main(argv=None, runner=None):
    ap = argparse.ArgumentParser(prog="bench.py", description="Benchmark harness for the contract checker.")
    sub = ap.add_subparsers(dest="cmd", required=True)
    rp = sub.add_parser("run", help="run the pipeline on every case of the corpora")
    rp.add_argument("--corpus", action="append", help="corpus directory (repeatable; default bench/corpus)")
    rp.add_argument("--cli", default=str(DEFAULT_CLI))
    rp.add_argument("--checker", default=str(DEFAULT_CHECKER))
    rp.add_argument("--out", help="write the result JSON here")
    rp.add_argument("--compare", help="previous result JSON to compare with")
    rp.add_argument("--format", choices=("text", "markdown"), default="text")
    rp.add_argument("--keep-work", help="keep extracted trees and databases in this directory")
    args = ap.parse_args(argv)
    try:
        corpora = [load_corpus(d) for d in (args.corpus or [DEFAULT_CORPUS])]
        prev = None
        if args.compare:
            try:
                prev = json.loads(Path(args.compare).read_text())
            except (OSError, json.JSONDecodeError) as e:
                raise BenchError(f"{args.compare}: cannot read previous result: {e}")
            if prev.get("schema") != 1:
                raise BenchError(f"{args.compare}: unsupported schema {prev.get('schema')}")
        if args.keep_work:
            workdir = Path(args.keep_work)
            workdir.mkdir(parents=True, exist_ok=True)
            tmp = None
        else:
            tmp = tempfile.TemporaryDirectory(prefix="bench-")
            workdir = Path(tmp.name)
        try:
            if runner is None:
                for b in (args.cli, args.checker):
                    if not os.access(b, os.X_OK):
                        raise BenchError(f"missing {b} (build it first)")
                runner = make_runner(Path(args.cli).resolve(), Path(args.checker).resolve(), workdir)
            result = run_bench(corpora, runner, workdir)
        finally:
            if tmp is not None:
                tmp.cleanup()
    except BenchError as e:
        print(f"bench: error: {e}", file=sys.stderr)
        return 2
    print(render(result, args.format))
    if args.out:
        Path(args.out).write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n")
    regression = False
    if prev is not None:
        lines, regression = compare(prev, result)
        print()
        print("\n".join(lines) if lines else "no change from " + args.compare)
    failed = result["summary"]["FAILED"] or any(r.get("failed") for r in result["labelled_runs"])
    return 2 if failed else 1 if regression else 0


if __name__ == "__main__":
    sys.exit(main())
