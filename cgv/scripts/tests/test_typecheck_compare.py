# Tests for scripts/typecheck_compare.py. Run: python3 -m unittest discover -s scripts/tests
import contextlib
import importlib.util
import io
import json
import subprocess
import tempfile
import textwrap
import unittest
from pathlib import Path
from unittest import mock

SCRIPTS = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("typecheck_compare", SCRIPTS / "typecheck_compare.py")
tc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(tc)


class MypyLineTest(unittest.TestCase):
    def test_parses_file_line_and_code(self):
        m = tc.MYPY_LINE.match('fleet/logic.py:6: error: Item "None" of "Vehicle | None" has no attribute "name"  [union-attr]')
        self.assertEqual((m["file"], m["line"], m["code"]), ("fleet/logic.py", "6", "union-attr"))

    def test_ignores_notes(self):
        self.assertIsNone(tc.MYPY_LINE.match("fleet/logic.py:6: note: See https://example.org"))


class CellTest(unittest.TestCase):
    FILES = ["app/logic.py"]

    def test_error_removed_by_fix_is_flagged_even_when_lines_move(self):
        pre = [("app/logic.py", 6, "union-attr", "a")]
        fix = [("app/logic.py", 4, "type-arg", "b")]
        self.assertEqual(tc.cell(pre, fix, self.FILES), "flagged: line 6 `union-attr`")

    def test_error_kept_by_fix_is_not_flagged_even_when_message_changes(self):
        pre = [("app/logic.py", 6, "reportUnknownArgumentType", "Argument type is unknown")]
        fix = [("app/logic.py", 9, "reportUnknownArgumentType", "Argument type is partially unknown")]
        self.assertEqual(tc.cell(pre, fix, self.FILES), "not flagged")

    def test_one_of_two_errors_with_the_same_rule_removed_is_flagged(self):
        pre = [("app/logic.py", 5, "misc", "x"), ("app/logic.py", 7, "misc", "y")]
        fix = [("app/logic.py", 5, "misc", "x")]
        self.assertEqual(tc.cell(pre, fix, self.FILES), "flagged: line 7 `misc`")

    def test_error_in_another_file_is_not_flagged(self):
        pre = [("app/models.py", 3, "misc", "x")]
        self.assertEqual(tc.cell(pre, [], self.FILES), "not flagged")


class CorpusTest(unittest.TestCase):
    def test_reads_directory_cases_and_skips_git_cases(self):
        with tempfile.TemporaryDirectory() as tmp:
            cases = Path(tmp) / "cases"
            (cases / "a").mkdir(parents=True)
            (cases / "a" / "pre").mkdir()
            (cases / "a" / "fix").mkdir()
            (cases / "a" / "case.toml").write_text(textwrap.dedent("""\
                kind = "length"
                in_scope = true
                pre = "pre"
                fix = "fix"
                [[bug]]
                file = "app/logic.py"
                line = 5
                [[bug]]
                file = "app/other.py"
                line = 2
                [[bug]]
                file = "app/e.py"
                line = 3
                [[bug]]
                file = "app/d.py"
                line = 3
                [[bug]]
                file = "app/c.py"
                line = 3
                [[bug]]
                file = "app/b.py"
                line = 3
                """))
            (cases / "b").mkdir()
            (cases / "b" / "case.toml").write_text(textwrap.dedent("""\
                kind = "length"
                in_scope = true
                pre_rev = "x^"
                fix_rev = "x"
                [[bug]]
                file = "app/logic.py"
                line = 5
                """))
            loaded = tc.load_cases(Path(tmp))
        self.assertEqual([c["id"] for c in loaded], ["a"])
        self.assertEqual(
            loaded[0]["files"],
            ["app/b.py", "app/c.py", "app/d.py", "app/e.py", "app/logic.py", "app/other.py"],
        )
        self.assertEqual(loaded[0]["pre"].name, "pre")

    def test_case_toml_id_overrides_the_directory_name(self):
        with tempfile.TemporaryDirectory() as tmp:
            case_dir = Path(tmp) / "cases" / "dirname"
            case_dir.mkdir(parents=True)
            (case_dir / "pre").mkdir()
            (case_dir / "fix").mkdir()
            (case_dir / "case.toml").write_text(
                'id = "explicit"\nkind = "k"\nin_scope = false\npre = "pre"\nfix = "fix"\n'
                '[[bug]]\nfile = "a.py"\nline = 1\n'
            )
            loaded = tc.load_cases(Path(tmp))
        self.assertEqual([(c["id"], c["in_scope"]) for c in loaded], [("explicit", False)])

    def test_a_case_missing_a_required_key_is_a_harness_error(self):
        with tempfile.TemporaryDirectory() as tmp:
            case_dir = Path(tmp) / "cases" / "broken"
            case_dir.mkdir(parents=True)
            (case_dir / "case.toml").write_text('kind = "k"\npre = "pre"\nfix = "fix"\n[[bug]]\nfile = "a.py"\nline = 1\n')
            with self.assertRaisesRegex(tc.HarnessError, "broken.*malformed"):
                tc.load_cases(Path(tmp))

    def write_case(self, tmp, text):
        case_dir = Path(tmp) / "cases" / "broken"
        case_dir.mkdir(parents=True)
        (case_dir / "case.toml").write_text(text)

    def test_invalid_toml_is_a_harness_error(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.write_case(tmp, "kind = = oops")
            with self.assertRaisesRegex(tc.HarnessError, "broken.*malformed"):
                tc.load_cases(Path(tmp))

    def test_a_case_toml_that_cannot_be_read_is_a_harness_error(self):
        with tempfile.TemporaryDirectory() as tmp:
            (Path(tmp) / "cases" / "broken" / "case.toml").mkdir(parents=True)
            with self.assertRaisesRegex(tc.HarnessError, "broken.*malformed"):
                tc.load_cases(Path(tmp))

    def test_a_case_toml_that_is_not_utf8_is_a_harness_error(self):
        with tempfile.TemporaryDirectory() as tmp:
            case_dir = Path(tmp) / "cases" / "broken"
            case_dir.mkdir(parents=True)
            (case_dir / "case.toml").write_bytes(b"\xff\xfe=")
            with self.assertRaisesRegex(tc.HarnessError, "broken.*malformed"):
                tc.load_cases(Path(tmp))

    def case_with(self, tmp, extra="", bug='[[bug]]\nfile = "a.py"\n', trees=("pre", "fix"),
                  kind='"k"', in_scope="true"):
        case_dir = Path(tmp) / "cases" / "broken"
        case_dir.mkdir(parents=True)
        for t in trees:
            (case_dir / t).mkdir()
        (case_dir / "case.toml").write_text(
            f'kind = {kind}\nin_scope = {in_scope}\npre = "pre"\nfix = "fix"\n{extra}\n{bug}')

    def test_a_non_string_id_is_a_harness_error(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.case_with(tmp, extra="id = []")
            with self.assertRaisesRegex(tc.HarnessError, "broken.*malformed"):
                tc.load_cases(Path(tmp))

    def test_malformed_field_values_are_harness_errors(self):
        for name, kwargs in (
            ("empty id", {"extra": 'id = ""'}),
            ("list kind", {"kind": "[]"}),
            ("string in_scope", {"in_scope": '"false"'}),
            ("int file", {"bug": "[[bug]]\nfile = 1\n"}),
            ("empty file", {"bug": '[[bug]]\nfile = ""\n'}),
        ):
            with self.subTest(name), tempfile.TemporaryDirectory() as tmp:
                self.case_with(tmp, **kwargs)
                with self.assertRaisesRegex(tc.HarnessError, "broken.*malformed"):
                    tc.load_cases(Path(tmp))

    def test_a_deeply_nested_case_toml_is_a_harness_error(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.write_case(tmp, "a = " + "[" * 100000)
            with self.assertRaisesRegex(tc.HarnessError, "broken.*malformed"):
                tc.load_cases(Path(tmp))

    def test_an_empty_bug_list_is_a_harness_error(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.case_with(tmp, bug="bug = []")
            with self.assertRaisesRegex(tc.HarnessError, "broken.*malformed"):
                tc.load_cases(Path(tmp))

    def test_a_missing_tree_is_a_harness_error(self):
        for trees in ((), ("pre",), ("fix",)):
            with self.subTest(trees), tempfile.TemporaryDirectory() as tmp:
                self.case_with(tmp, trees=trees)
                with self.assertRaisesRegex(tc.HarnessError, "broken.*malformed"):
                    tc.load_cases(Path(tmp))

    def test_a_well_formed_case_with_both_trees_loads(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.case_with(tmp)
            loaded = tc.load_cases(Path(tmp))
        self.assertEqual([(c["id"], c["files"]) for c in loaded], [("broken", ["a.py"])])

    def test_a_bug_table_instead_of_an_array_is_a_harness_error(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.write_case(tmp, 'kind = "k"\nin_scope = true\npre = "p"\nfix = "f"\n[bug]\nfile = "a.py"\n')
            with self.assertRaisesRegex(tc.HarnessError, "broken.*malformed"):
                tc.load_cases(Path(tmp))

    def test_settings_install_every_package_with_models(self):
        with tempfile.TemporaryDirectory() as tmp:
            tree = Path(tmp)
            for pkg in ("fleet", "billing"):
                (tree / pkg).mkdir()
                (tree / pkg / "models.py").write_text("")
            (tree / "util").mkdir()
            (tree / "util" / "logic.py").write_text("")
            tc.write_config(tree, "python3")
            settings = (tree / f"{tc.SETTINGS_MODULE}.py").read_text()
        self.assertIn("INSTALLED_APPS = ['billing', 'fleet']", settings)

    def test_both_checkers_are_configured_strict(self):
        with tempfile.TemporaryDirectory() as tmp:
            tree = Path(tmp)
            tc.write_config(tree, "python3")
            mypy_ini = (tree / "mypy.ini").read_text().splitlines()
            pyright = json.loads((tree / "pyrightconfig.json").read_text())
        self.assertIn("strict = True", mypy_ini)
        self.assertIn("plugins = mypy_django_plugin.main, pydantic.mypy", mypy_ini)
        self.assertIn(f"django_settings_module = {tc.SETTINGS_MODULE}", mypy_ini)
        self.assertIn(f"exclude = {tc.SETTINGS_MODULE}\\.py", mypy_ini)
        self.assertEqual(pyright, {"typeCheckingMode": "strict", "exclude": [f"{tc.SETTINGS_MODULE}.py"]})


def proc(returncode=0, stdout="", stderr=""):
    return subprocess.CompletedProcess([], returncode, stdout, stderr)


class RunnerTest(unittest.TestCase):
    def test_mypy_keeps_errors_and_drops_notes(self):
        out = (
            "app/logic.py:6: error: Item is None  [union-attr]\n"
            "app/logic.py:6: note: See docs\n"
            "app/models.py:12: error: Bad thing\n"
        )
        with mock.patch.object(tc.subprocess, "run", return_value=proc(1, out)):
            diags = tc.run_mypy(Path("."), "python3")
        self.assertEqual(diags, [
            ("app/logic.py", 6, "union-attr", "Item is None"),
            ("app/models.py", 12, "", "Bad thing"),
        ])

    def test_mypy_exit_two_is_a_harness_error(self):
        with mock.patch.object(tc.subprocess, "run", return_value=proc(2, "", "boom")):
            with self.assertRaisesRegex(tc.HarnessError, "mypy failed"):
                tc.run_mypy(Path("."), "python3")

    def test_pyright_keeps_errors_with_relative_paths_and_one_based_lines(self):
        with tempfile.TemporaryDirectory() as tmp:
            tree = Path(tmp)
            out = json.dumps({"generalDiagnostics": [
                {"file": str(tree / "app" / "logic.py"), "severity": "error",
                 "rule": "reportOptionalMemberAccess", "message": "m",
                 "range": {"start": {"line": 5, "character": 0}}},
                {"file": str(tree / "app" / "logic.py"), "severity": "warning",
                 "rule": "reportUnused", "message": "w",
                 "range": {"start": {"line": 7, "character": 0}}},
                {"file": str(tree / "app" / "x.py"), "severity": "error",
                 "message": "no rule", "range": {"start": {"line": 0, "character": 0}}},
            ]})
            with mock.patch.object(tc.subprocess, "run", return_value=proc(1, out)):
                diags = tc.run_pyright(tree, "python3")
        self.assertEqual(diags, [
            ("app/logic.py", 6, "reportOptionalMemberAccess", "m"),
            ("app/x.py", 1, "", "no rule"),
        ])

    def test_pyright_output_that_is_not_json_is_a_harness_error(self):
        with mock.patch.object(tc.subprocess, "run", return_value=proc(1, "Traceback", "err")):
            with self.assertRaisesRegex(tc.HarnessError, "pyright failed"):
                tc.run_pyright(Path("."), "python3")

    def test_a_missing_package_is_named_in_the_harness_error(self):
        def fake(cmd, **kw):
            return proc(1 if "'django-stubs'" in cmd[2] else 0, "9.9")
        with mock.patch.object(tc.subprocess, "run", side_effect=fake):
            with self.assertRaisesRegex(tc.HarnessError, "django-stubs is not installed"):
                tc.versions("python3")

    def test_versions_reads_each_pinned_package(self):
        with mock.patch.object(tc.subprocess, "run", return_value=proc(0, "9.9\n")):
            out = tc.versions("python3")
        self.assertEqual(out, {p: "9.9" for p in ("mypy", "pyright", "django-stubs", "django", "pydantic")})


class MainTest(unittest.TestCase):
    VERSIONS = {"mypy": "1", "pyright": "2", "django-stubs": "3", "django": "4", "pydantic": "5"}

    def corpus(self, tmp, in_scope):
        case_dir = Path(tmp) / "cases" / "c1"
        (case_dir / "pre").mkdir(parents=True)
        (case_dir / "fix").mkdir()
        (case_dir / "case.toml").write_text(
            f'kind = "non-null"\nin_scope = {str(in_scope).lower()}\npre = "pre"\nfix = "fix"\n'
            '[[bug]]\nfile = "app/logic.py"\nline = 1\n'
        )
        result = Path(tmp) / "result.json"
        result.write_text(json.dumps({"cases": [{"id": "c1", "outcome": "CAUGHT"}]}))
        return Path(tmp), result

    def run_main(self, tmp, in_scope, mypy_runs, bench_result=None):
        corpus, result = self.corpus(tmp, in_scope)
        out, err = io.StringIO(), io.StringIO()
        with mock.patch.object(tc, "versions", return_value=self.VERSIONS), \
                mock.patch.object(tc, "run_mypy", side_effect=mypy_runs), \
                mock.patch.object(tc, "run_pyright", return_value=[]), \
                contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = tc.main(["--corpus", str(corpus), "--bench-result", str(bench_result or result)])
        return code, out.getvalue(), err.getvalue()

    def test_prints_versions_and_one_row_per_case(self):
        with tempfile.TemporaryDirectory() as tmp:
            code, out, _ = self.run_main(
                tmp, True, [[("app/logic.py", 3, "union-attr", "m")], []])
        self.assertEqual(code, 0)
        self.assertIn("mypy 1 (`strict`, django-stubs 3, pydantic plugin), pyright 2 (`strict`), Django 4, pydantic 5.", out)
        self.assertIn("| `c1` | non-null | yes | CAUGHT | flagged: line 3 `union-attr` | not flagged |", out)

    def test_a_case_outside_the_design_reach_is_labelled_not_yet_reached(self):
        with tempfile.TemporaryDirectory() as tmp:
            _, out, _ = self.run_main(tmp, False, [[], []])
        self.assertIn("| `c1` | non-null | not yet reached | CAUGHT |", out)

    def test_a_case_missing_from_the_bench_result_prints_n_a(self):
        with tempfile.TemporaryDirectory() as tmp:
            other = Path(tmp) / "other.json"
            other.write_text(json.dumps({"cases": [{"id": "zzz", "outcome": "CAUGHT"}]}))
            _, out, _ = self.run_main(tmp, True, [[], []], bench_result=other)
        self.assertIn("| `c1` | non-null | yes | n/a |", out)

    def test_a_harness_error_exits_two_and_prints_nothing_to_stdout(self):
        with tempfile.TemporaryDirectory() as tmp:
            code, out, err = self.run_main(tmp, True, tc.HarnessError("mypy failed in x"))
        self.assertEqual((code, out), (2, ""))
        self.assertIn("error: mypy failed in x", err)

    def test_a_malformed_bench_result_exits_two(self):
        for name, text in (("key", json.dumps({"cases": [{"id": "c1"}]})),
                           ("json", "not json"),
                           ("list", "[]"),
                           ("cases-int", json.dumps({"cases": 5})),
                           ("case-int", json.dumps({"cases": [5]})),
                           ("utf8", b"\xff\xfe")):
            with self.subTest(name), tempfile.TemporaryDirectory() as tmp:
                bad = Path(tmp) / "bad.json"
                (bad.write_bytes if isinstance(text, bytes) else bad.write_text)(text)
                code, out, err = self.run_main(tmp, True, [[], []], bench_result=bad)
                self.assertEqual((code, out), (2, ""))
                self.assertIn("malformed", err)

    def test_a_missing_bench_result_exits_two(self):
        with tempfile.TemporaryDirectory() as tmp:
            code, out, err = self.run_main(tmp, True, [[], []], bench_result=Path(tmp) / "absent.json")
        self.assertEqual((code, out), (2, ""))
        self.assertIn("malformed", err)


if __name__ == "__main__":
    unittest.main()
