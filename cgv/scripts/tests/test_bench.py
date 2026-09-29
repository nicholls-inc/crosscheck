# Tests for scripts/bench.py. Run: python3 -m unittest discover -s scripts/tests
import contextlib
import importlib.util
import io
import json
import os
import shutil
import subprocess
import tempfile
import textwrap
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("bench", SCRIPTS / "bench.py")
bench = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bench)


def res(severity, req, file, line, source="f", target="M.x", hop=None, status="inconsistent"):
    return {"status": status, "severity": severity, "source": {"name": source}, "target": {"name": target},
            "hop": hop or [source, target], "site": {"file": file, "line": line} if file else None,
            "target_requirement": req}


def out(*results, code=None):
    if code is None:
        code = 1 if any(r["severity"] == "error" for r in results) else 0
    return code, {"results": list(results)}


def case(kind="length", in_scope=True, bugs=None):
    return {"id": "c", "kind": kind, "in_scope": in_scope,
            "bugs": bugs or [{"file": "app/views.py", "lo": 10, "hi": 10, "target": None}]}


class TempCorpus:
    """A corpus written into a temp dir, with a fake runner keyed on '<case>/<side>'."""

    def __init__(self, test):
        self.dir = Path(tempfile.mkdtemp(prefix="bench-test-"))
        test.addCleanup(shutil.rmtree, self.dir)
        self.outputs = {}
        self.calls = []
        self.write("corpus.toml", 'name = "t"\n')

    def write(self, rel, text):
        p = self.dir / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(textwrap.dedent(text))
        return p

    def dir_case(self, cid, toml, pre=None, fix=None):
        self.write(f"cases/{cid}/case.toml", 'pre = "pre"\nfix = "fix"\n' + textwrap.dedent(toml))
        (self.dir / f"cases/{cid}/pre").mkdir(exist_ok=True)
        (self.dir / f"cases/{cid}/fix").mkdir(exist_ok=True)
        if pre is not None:
            self.outputs[f"{cid}/pre"] = pre
        if fix is not None:
            self.outputs[f"{cid}/fix"] = fix

    def runner(self, app_dir, args):
        app_dir = Path(app_dir)
        self.calls.append((app_dir, list(args)))
        return self.outputs[f"{app_dir.parent.name}/{app_dir.name}"]

    def main(self, *extra, runner=None):
        stdout, stderr = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            code = bench.main(["run", "--corpus", str(self.dir), *extra], runner=runner or self.runner)
        return code, stdout.getvalue(), stderr.getvalue()


LEN_BUG = 'kind = "length"\nin_scope = true\n[[bug]]\nfile = "app/views.py"\nline = 10\n'


class MatchingTest(unittest.TestCase):
    def bug(self, file="app/views.py", lo=10, hi=10, target=None):
        return {"file": file, "lo": lo, "hi": hi, "target": target}

    def test_exact_file_and_line(self):
        self.assertTrue(bench.matches(res("error", "length ≤ 5", "app/views.py", 10), self.bug(), "length"))

    def test_file_suffix_either_way(self):
        r = res("error", "length ≤ 5", "views.py", 10)
        self.assertTrue(bench.matches(r, self.bug(), "length"))
        r = res("error", "length ≤ 5", "src/app/views.py", 10)
        self.assertTrue(bench.matches(r, self.bug(), "length"))

    def test_partial_name_suffix_is_not_a_match(self):
        r = res("error", "length ≤ 5", "myviews.py", 10)
        self.assertFalse(bench.matches(r, self.bug(file="views.py"), "length"))

    def test_line_range_inclusive(self):
        b = self.bug(lo=10, hi=14)
        self.assertEqual([bench.matches(res("error", "length ≤ 5", "app/views.py", n), b, "length")
                          for n in (9, 10, 12, 14, 15)], [False, True, True, True, False])

    def test_target_substring(self):
        r = res("error", "length ≤ 5", "app/views.py", 10, target="Meter.first_line_address")
        self.assertTrue(bench.matches(r, self.bug(target="first_line"), "length"))
        self.assertFalse(bench.matches(r, self.bug(target="Meter.postcode"), "length"))

    def test_kind_is_first_token_of_requirement(self):
        r = res("error", "non-null", "app/views.py", 10)
        self.assertTrue(bench.matches(r, self.bug(), "non-null"))
        self.assertFalse(bench.matches(r, self.bug(), "length"))
        r = res("error", "choices in [a, b]", "app/views.py", 10)
        self.assertTrue(bench.matches(r, self.bug(), "choices"))

    def test_no_site_never_matches(self):
        self.assertFalse(bench.matches(res("error", "length ≤ 5", None, None), self.bug(), "length"))


class ClassifyTest(unittest.TestCase):
    def test_caught(self):
        pre = [res("error", "length ≤ 5", "app/views.py", 10)]
        outcome, rank, matched = bench.classify(case(), pre, [])
        self.assertEqual((outcome, rank, len(matched)), ("CAUGHT", [1, 1], 1))

    def test_detected_not_cleared(self):
        pre = [res("error", "length ≤ 5", "app/views.py", 10)]
        fix = [res("error", "length ≤ 5", "app/views.py", 10)]
        self.assertEqual(bench.classify(case(), pre, fix)[0], "DETECTED_NOT_CLEARED")

    def test_fix_matched_by_identity_when_lines_shift(self):
        pre = [res("error", "length ≤ 5", "app/views.py", 10)]
        shifted = [res("error", "length ≤ 5", "app/views.py", 17)]
        self.assertEqual(bench.classify(case(), pre, shifted)[0], "DETECTED_NOT_CLEARED")

    def test_fix_error_with_other_identity_still_caught(self):
        pre = [res("error", "length ≤ 5", "app/views.py", 10)]
        fix = [res("error", "length ≤ 5", "app/views.py", 10, target="M.other"),
               res("warning", "length ≤ 5", "app/views.py", 10)]
        self.assertEqual(bench.classify(case(), pre, fix)[0], "CAUGHT")

    def test_warning_only_with_warning_rank(self):
        pre = [res("error", "length ≤ 5", "app/a.py", 1),
               res("warning", "length ≤ 5", "app/a.py", 3),
               res("warning", "length ≤ 5", "app/views.py", 10)]
        outcome, rank, matched = bench.classify(case(), pre, [])
        self.assertEqual((outcome, rank), ("WARNING_ONLY", [2, 2]))
        self.assertEqual([m["severity"] for m in matched], ["warning"])

    def test_missed(self):
        pre = [res("error", "precision ≤ 2", "app/views.py", 10)]
        self.assertEqual(bench.classify(case(), pre, [])[:2], ("MISSED", None))

    def test_incomplete_is_neither_error_nor_warning(self):
        pre = [res("error", "length ≤ 5", "app/views.py", 10, status="incomplete")]
        self.assertEqual(bench.classify(case(), pre, [])[0], "MISSED")

    def test_out_of_scope_not_reported_records_warnings(self):
        pre = [res("warning", "length ≤ 5", "app/views.py", 10)]
        outcome, rank, matched = bench.classify(case(in_scope=False), pre, [])
        self.assertEqual((outcome, rank, len(matched)), ("NOT_REPORTED", None, 1))

    def test_out_of_scope_spurious_error(self):
        pre = [res("error", "length ≤ 5", "app/views.py", 10)]
        self.assertEqual(bench.classify(case(in_scope=False), pre, [])[0], "SPURIOUS_ERROR")

    def test_rank_sorted_by_file_line_target_source(self):
        pre = [res("error", "length ≤ 5", "app/z.py", 1),
               res("error", "length ≤ 5", "app/views.py", 10, target="M.y"),
               res("error", "length ≤ 5", "app/views.py", 10, target="M.b"),
               res("error", "length ≤ 5", "app/views.py", 2),
               res("error", "length ≤ 5", "app/a.py", 99)]
        c = case(bugs=[{"file": "app/views.py", "lo": 10, "hi": 10, "target": "M.y"}])
        self.assertEqual(bench.classify(c, pre, [])[1], [4, 5])

    def test_any_bug_may_match(self):
        c = case(bugs=[{"file": "app/a.py", "lo": 1, "hi": 1, "target": None},
                       {"file": "app/b.py", "lo": 5, "hi": 6, "target": None}])
        pre = [res("error", "length ≤ 5", "app/b.py", 6)]
        self.assertEqual(bench.classify(c, pre, [])[0], "CAUGHT")


class RunTest(unittest.TestCase):
    def test_outcomes_counts_and_args(self):
        t = TempCorpus(self)
        t.write("corpus.toml", 'name = "t"\ncheck_args = ["--exclude", "**/tests/**"]\n')
        t.dir_case("a", 'check_args = ["--allow-parse-errors"]\n' + LEN_BUG,
                   pre=out(res("error", "length ≤ 5", "app/views.py", 10),
                           res("warning", "length ≤ 5", "app/x.py", 1)),
                   fix=out(res("warning", "length ≤ 5", "app/x.py", 1)))
        t.dir_case("b", LEN_BUG, pre=out(), fix=out())
        code, stdout, _ = t.main()
        self.assertEqual(code, 0)
        self.assertEqual(t.calls[0][1], ["--exclude", "**/tests/**", "--allow-parse-errors"])
        self.assertEqual(t.calls[2][1], ["--exclude", "**/tests/**"])
        self.assertIn("a     length  in     CAUGHT   1 of 1  1/1      0/1", stdout)
        self.assertIn("in scope 2: CAUGHT 1, DETECTED_NOT_CLEARED 0, WARNING_ONLY 0, MISSED 1", stdout)

    def test_failed_pipeline_exit_2(self):
        t = TempCorpus(self)
        t.dir_case("a", LEN_BUG, pre=(2, None), fix=out())
        t.dir_case("b", LEN_BUG, pre=out(), fix=(1, None))
        result = t.dir / "r.json"
        code, stdout, _ = t.main("--out", str(result))
        self.assertEqual(code, 2)
        data = json.loads(result.read_text())
        self.assertEqual([c["outcome"] for c in data["cases"]], ["FAILED", "FAILED"])
        self.assertEqual(data["cases"][0]["pre"], {"exit": 2, "errors": None, "warnings": None})
        self.assertEqual(data["summary"]["FAILED"], 2)

    def test_markdown_format(self):
        t = TempCorpus(self)
        t.dir_case("a", LEN_BUG, pre=out(), fix=out())
        _, stdout, _ = t.main("--format", "markdown")
        self.assertIn("| case | kind | scope | outcome | rank | pre e/w | fix e/w |", stdout)
        self.assertIn("| a | length | in | MISSED | - | 0/0 | 0/0 |", stdout)

    def test_duplicate_ids_across_corpora(self):
        t1, t2 = TempCorpus(self), TempCorpus(self)
        t1.dir_case("a", LEN_BUG, pre=out(), fix=out())
        t2.dir_case("a", LEN_BUG, pre=out(), fix=out())
        code, _, stderr = t1.main("--corpus", str(t2.dir))
        self.assertEqual(code, 2)
        self.assertIn("duplicate case id 'a'", stderr)

    def test_result_has_no_absolute_paths_or_timestamps(self):
        t = TempCorpus(self)
        t.dir_case("a", LEN_BUG, pre=out(res("error", "length ≤ 5", "app/views.py", 10)), fix=out())
        result = t.dir / "r.json"
        t.main("--out", str(result))
        text = result.read_text()
        self.assertNotIn(str(t.dir), text)
        self.assertNotIn(str(bench.REPO_ROOT), text)

        def strings(o):
            if isinstance(o, dict):
                for k, v in o.items():
                    yield k
                    yield from strings(v)
            elif isinstance(o, list):
                for v in o:
                    yield from strings(v)
            elif isinstance(o, str):
                yield o
        for s in strings(json.loads(text)):
            self.assertFalse(s.startswith("/"), s)
            self.assertNotRegex(s, r"\d{4}-\d{2}-\d{2}|time|date")


class MalformedCaseTest(unittest.TestCase):
    def check(self, toml, message):
        t = TempCorpus(self)
        t.write("cases/bad/case.toml", toml)
        code, stdout, stderr = t.main()
        self.assertEqual(code, 2)
        self.assertIn(message, stderr)
        self.assertIn("cases/bad/case.toml", stderr)
        self.assertEqual(t.calls, [])

    def test_missing_kind(self):
        self.check('in_scope = true\npre = "p"\nfix = "f"\n[[bug]]\nfile = "a.py"\nline = 1\n',
                   "missing 'kind'")

    def test_unknown_kind(self):
        self.check('kind = "size"\nin_scope = true\n', "unknown kind 'size'")

    def test_dir_and_git_keys(self):
        self.check('kind = "length"\nin_scope = true\npre = "pre"\nfix = "fix"\npre_rev = "a^"\n'
                   'fix_rev = "a"\n[[bug]]\nfile = "a.py"\nline = 1\n', "both directory keys")

    def test_missing_bug(self):
        t = TempCorpus(self)
        t.dir_case("bad", 'kind = "length"\nin_scope = true\n')
        code, _, stderr = t.main()
        self.assertEqual(code, 2)
        self.assertIn("missing [[bug]]", stderr)

    def test_bug_needs_line_or_lines(self):
        t = TempCorpus(self)
        t.dir_case("bad", 'kind = "length"\nin_scope = true\n[[bug]]\nfile = "a.py"\n')
        code, _, stderr = t.main()
        self.assertEqual(code, 2)
        self.assertIn("[[bug]] #1: give exactly one of 'line' or 'lines'", stderr)


class ValidationTest(unittest.TestCase):
    def test_reversed_line_range(self):
        t = TempCorpus(self)
        t.dir_case("bad", 'kind = "length"\nin_scope = true\n[[bug]]\nfile = "a.py"\nlines = [14, 10]\n')
        code, _, stderr = t.main()
        self.assertEqual(code, 2)
        self.assertIn("first <= last", stderr)

    def test_line_and_lines_together(self):
        t = TempCorpus(self)
        t.dir_case("bad", 'kind = "length"\nin_scope = true\n[[bug]]\nfile = "a.py"\nline = 1\nlines = [1, 2]\n')
        code, _, stderr = t.main()
        self.assertEqual(code, 2)
        self.assertIn("exactly one of 'line' or 'lines'", stderr)

    def test_case_id_cannot_escape_workdir(self):
        t = TempCorpus(self)
        t.dir_case("bad", 'id = "../x"\n' + LEN_BUG)
        code, _, stderr = t.main()
        self.assertEqual(code, 2)
        self.assertIn("case id '../x'", stderr)

    def test_duplicate_label_identity(self):
        t = TempCorpus(self)
        line = json.dumps({"run": "lab", "source": "f", "target": "M.x", "hop": ["f", "M.x"], "kind": "length",
                           "site_file": "a.py", "label": "benign"})
        t.write("labels.jsonl", line + "\n" + line + "\n")
        code, _, stderr = t.main()
        self.assertEqual(code, 2)
        self.assertIn("labels.jsonl:2: duplicate label", stderr)


class LabelTest(unittest.TestCase):
    def label(self, source, label, cause="", target="M.x", site_file="app/a.py", kind="length"):
        return json.dumps({"run": "lab", "source": source, "target": target, "hop": [source, target],
                           "kind": kind, "site_file": site_file, "label": label, "cause": cause, "note": ""})

    def test_join_counts_precision_unlabelled_stale(self):
        t = TempCorpus(self)
        t.write("corpus.toml", 'name = "t"\n[[labelled_run]]\nid = "lab"\napp = "labelled/app"\n')
        (t.dir / "labelled/app").mkdir(parents=True)
        t.write("labels.jsonl", "\n".join([
            self.label("f", "true_bug"),
            self.label("g", "false_positive", "numeric-tower"),
            self.label("h", "false_positive", "numeric-tower"),
            self.label("i", "benign"),
            self.label("gone", "false_positive", "no-return"),
            self.label("f", "true_bug", site_file="app/other.py"),
        ]) + "\n")
        t.outputs["labelled/app"] = out(
            res("error", "length ≤ 5", "app/a.py", 1, source="f"),
            res("error", "length ≤ 5", "app/a.py", 40, source="f"),  # same identity, other line
            res("error", "length ≤ 5", "app/a.py", 2, source="g"),
            res("error", "length ≤ 5", "app/a.py", 3, source="h"),
            res("error", "length ≤ 5", "app/a.py", 4, source="i"),
            res("error", "length ≤ 5", "app/a.py", 5, source="new"),
            res("warning", "length ≤ 5", "app/a.py", 6, source="w"))
        result = t.dir / "r.json"
        code, stdout, _ = t.main("--out", str(result))
        self.assertEqual(code, 0)
        self.assertEqual(json.loads(result.read_text())["labelled_runs"], [{
            "id": "lab", "corpus": "t", "errors": 6, "labelled": 5, "true_bug": 2, "benign": 1,
            "false_positive": 2, "precision": 0.4, "unlabelled": 1, "stale_labels": 2,
            "false_positive_causes": {"numeric-tower": 2}}])
        self.assertIn("lab false positive causes: numeric-tower 2", stdout)

    def test_bad_label_value(self):
        t = TempCorpus(self)
        t.write("labels.jsonl", self.label("f", "maybe") + "\n")
        code, _, stderr = t.main()
        self.assertEqual(code, 2)
        self.assertIn("labels.jsonl:1: unknown label 'maybe'", stderr)


class CompareTest(unittest.TestCase):
    def run_pair(self, before, after):
        t = TempCorpus(self)
        for cid, (pre, fix) in before.items():
            t.dir_case(cid, LEN_BUG, pre=pre, fix=fix)
        prev = t.dir / "prev.json"
        self.assertEqual(t.main("--out", str(prev))[0], 0)
        for cid in before:
            if cid not in after:
                shutil.rmtree(t.dir / "cases" / cid)
        for cid, (pre, fix) in after.items():
            t.dir_case(cid, LEN_BUG, pre=pre, fix=fix)
        return t.main("--compare", str(prev))

    hit = out(res("error", "length ≤ 5", "app/views.py", 10))
    two = out(res("error", "length ≤ 5", "app/a.py", 1), res("error", "length ≤ 5", "app/views.py", 10))

    def test_regression_exits_1(self):
        code, stdout, _ = self.run_pair({"a": (self.hit, out())}, {"a": (out(), out())})
        self.assertEqual(code, 1)
        self.assertIn("REGRESSION: a CAUGHT -> MISSED", stdout)

    def test_improvement(self):
        code, stdout, _ = self.run_pair({"a": (self.hit, self.hit)}, {"a": (self.hit, out())})
        self.assertEqual(code, 0)
        self.assertIn("improvement: a DETECTED_NOT_CLEARED -> CAUGHT", stdout)

    def test_rank_change(self):
        code, stdout, _ = self.run_pair({"a": (self.hit, out())}, {"a": (self.two, out())})
        self.assertEqual(code, 0)
        self.assertIn("rank changed: a 1 of 1 -> 2 of 2", stdout)

    def test_new_and_removed_cases(self):
        code, stdout, _ = self.run_pair({"a": (self.hit, out())}, {"b": (self.hit, out())})
        self.assertEqual(code, 0)
        self.assertIn("new case: b CAUGHT", stdout)
        self.assertIn("removed case: a (was CAUGHT)", stdout)

    def test_labelled_run_changes(self):
        prev = {"cases": [], "labelled_runs": [{"id": "lab", "precision": 0.5, "false_positive": 2,
                                                "stale_labels": 0}]}
        cur = {"cases": [], "labelled_runs": [{"id": "lab", "precision": 0.25, "false_positive": 3,
                                               "stale_labels": 1}]}
        lines, regression = bench.compare(prev, cur)
        self.assertFalse(regression)
        self.assertEqual(lines, ["labelled run lab: precision 0.5 -> 0.25",
                                 "labelled run lab: false_positive 2 -> 3",
                                 "labelled run lab: stale_labels 0 -> 1"])

    def test_recovering_from_failed_is_an_improvement_across_scopes(self):
        prev = {"cases": [{"id": "a", "in_scope": False, "outcome": "FAILED", "rank": None}]}
        cur = {"cases": [{"id": "a", "in_scope": True, "outcome": "MISSED", "rank": None}], "labelled_runs": []}
        lines, regression = bench.compare(prev, cur)
        self.assertFalse(regression)
        self.assertEqual(lines, ["improvement: a FAILED -> MISSED"])

    def test_failed_is_worst(self):
        prev = {"cases": [{"id": "a", "in_scope": False, "outcome": "SPURIOUS_ERROR", "rank": None}]}
        cur = {"cases": [{"id": "a", "in_scope": False, "outcome": "FAILED", "rank": None}], "labelled_runs": []}
        self.assertTrue(bench.compare(prev, cur)[1])


def git(repo, *args):
    subprocess.run(["git", "-C", str(repo), *args], check=True, capture_output=True,
                   env={**os.environ, "GIT_AUTHOR_NAME": "t", "GIT_AUTHOR_EMAIL": "t@example.com",
                        "GIT_COMMITTER_NAME": "t", "GIT_COMMITTER_EMAIL": "t@example.com"})


@unittest.skipUnless(shutil.which("git"), "git not installed")
class GitCaseTest(unittest.TestCase):
    def setUp(self):
        self.repo = Path(tempfile.mkdtemp(prefix="bench-repo-"))
        self.addCleanup(shutil.rmtree, self.repo)
        git(self.repo, "init", "-q")
        views = self.repo / "src/app/views.py"
        views.parent.mkdir(parents=True)
        views.write_text("BUG\n")
        git(self.repo, "add", ".")
        git(self.repo, "commit", "-q", "-m", "pre")
        views.write_text("FIXED\n")
        git(self.repo, "commit", "-q", "-am", "fix")

    def runner(self, app_dir, args):
        self.seen.append(sorted(p.relative_to(app_dir).as_posix() for p in Path(app_dir).rglob("*.py")))
        if (Path(app_dir) / "app/views.py").read_text() == "BUG\n":
            return out(res("error", "length ≤ 5", "app/views.py", 10))
        return out()

    def test_archive_revisions_with_env_repo_and_subdir(self):
        self.seen = []
        t = TempCorpus(self)
        t.write("corpus.toml", 'name = "t"\nrepo = "${BENCH_TEST_REPO}"\n')
        t.write("cases/g/case.toml", 'kind = "length"\nin_scope = true\npre_rev = "HEAD^"\n'
                'fix_rev = "HEAD"\napp_subdir = "src"\n[[bug]]\nfile = "app/views.py"\nline = 10\n')
        os.environ["BENCH_TEST_REPO"] = str(self.repo)
        self.addCleanup(os.environ.pop, "BENCH_TEST_REPO")
        result = t.dir / "r.json"
        code, _, _ = t.main("--out", str(result), runner=self.runner)
        self.assertEqual(code, 0)
        self.assertEqual(json.loads(result.read_text())["cases"][0]["outcome"], "CAUGHT")
        self.assertEqual(self.seen, [["app/views.py"], ["app/views.py"]])
        self.assertEqual(subprocess.run(["git", "-C", str(self.repo), "status", "--porcelain"],
                                        capture_output=True, text=True).stdout, "")

    def test_bad_revision_fails_case(self):
        self.seen = []
        t = TempCorpus(self)
        t.write("cases/g/case.toml", f'kind = "length"\nin_scope = true\nrepo = "{self.repo}"\n'
                'pre_rev = "nosuchrev"\nfix_rev = "HEAD"\napp_subdir = "src"\n[[bug]]\nfile = "app/views.py"\nline = 10\n')
        code, stdout, stderr = t.main(runner=self.runner)
        self.assertEqual(code, 2)
        self.assertIn("FAILED", stdout)
        self.assertIn("git archive nosuchrev", stderr)

    def test_unset_env_var(self):
        t = TempCorpus(self)
        t.write("corpus.toml", 'name = "t"\nrepo = "$BENCH_TEST_UNSET_VAR"\n')
        code, _, stderr = t.main()
        self.assertEqual(code, 2)
        self.assertIn("environment variable BENCH_TEST_UNSET_VAR", stderr)


BINARIES = os.access(bench.DEFAULT_CLI, os.X_OK) and os.access(bench.DEFAULT_CHECKER, os.X_OK)


@unittest.skipUnless(BINARIES, "release binaries not built")
class IntegrationTest(unittest.TestCase):
    def test_real_pipeline_catches_too_long_code(self):
        t = TempCorpus(self)
        t.dir_case("too-long", 'kind = "length"\nin_scope = true\n'
                   '[[bug]]\nfile = "views.py"\nline = 5\ntarget = "Voucher.code"\n')
        model = ("from django.db import models\n\n\nclass Voucher(models.Model):\n"
                 "    code = models.CharField(max_length=5)\n")
        view = 'from models import Voucher\n\n\ndef make_voucher():\n    return Voucher.objects.create(code="{}")\n'
        for side, value in (("pre", "toolongcode"), ("fix", "ok")):
            t.write(f"cases/too-long/{side}/models.py", model)
            t.write(f"cases/too-long/{side}/views.py", view.format(value))
        result = t.dir / "r.json"
        stdout, stderr = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            code = bench.main(["run", "--corpus", str(t.dir), "--out", str(result)])
        self.assertEqual(code, 0, stderr.getvalue())
        c = json.loads(result.read_text())["cases"][0]
        self.assertEqual((c["outcome"], c["rank"], c["pre"]["exit"], c["fix"]["exit"]), ("CAUGHT", [1, 1], 1, 0))


if __name__ == "__main__":
    unittest.main()
