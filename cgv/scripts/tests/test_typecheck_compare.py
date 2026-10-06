# Tests for scripts/typecheck_compare.py. Run: python3 -m unittest discover -s scripts/tests
import importlib.util
import tempfile
import textwrap
import unittest
from pathlib import Path

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
        self.assertEqual(loaded[0]["files"], ["app/logic.py", "app/other.py"])
        self.assertEqual(loaded[0]["pre"].name, "pre")

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


if __name__ == "__main__":
    unittest.main()
