# Tests for scripts/behavior_diff.py: the gate in compare() and check_baseline_shape(), and the digit
# convention of BD-4. They need neither Lean nor Django. Run: python3 -m unittest discover -s scripts/tests
import copy
import importlib.util
import unittest
from decimal import Decimal
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("behavior_diff", SCRIPTS / "behavior_diff.py")
behavior_diff = importlib.util.module_from_spec(spec)
spec.loader.exec_module(behavior_diff)

LIB_ONLY, MODEL_ONLY = behavior_diff.DIRECTIONS


def row(drawn, lib_only=0, model_only=0):
    return {"drawn": drawn, LIB_ONLY: lib_only, MODEL_ONLY: model_only, "disagreements": lib_only + model_only}


def baseline(counts, tracked=None):
    return {"seed": 1, "examples": 10, "counts": counts, "tracked_by": {"p": "TB-1"} if tracked is None else tracked}


BASE = baseline({"c1": {"p": {"drawn": 10, LIB_ONLY: 1, MODEL_ONLY: 1}},
                 "c2": {"p": {"drawn": 10, LIB_ONLY: 0, MODEL_ONLY: 0}}})


def compare(summary, base=BASE, seed=1, examples=10, full=True):
    return behavior_diff.compare(summary, base, seed, examples, full=full)


class CompareTest(unittest.TestCase):
    def test_counts_equal_to_the_baseline_pass(self):
        self.assertEqual(compare({"c1": {"p": row(10, 1, 1)}, "c2": {"p": row(10)}}), [])

    def test_a_direction_moving_at_an_equal_total_fails_in_both_directions(self):
        problems = compare({"c1": {"p": row(10, 2, 0)}, "c2": {"p": row(10)}})
        self.assertEqual(len(problems), 2)
        self.assertTrue(any(LIB_ONLY in p and "2, baseline 1" in p for p in problems))
        self.assertTrue(any(MODEL_ONLY in p and "0, baseline 1" in p for p in problems))

    def test_fewer_disagreements_than_the_baseline_fail(self):
        self.assertEqual(len(compare({"c1": {"p": row(10, 0, 1)}, "c2": {"p": row(10)}})), 1)

    def test_a_change_in_the_inputs_drawn_fails(self):
        problems = compare({"c1": {"p": row(9, 1, 1)}, "c2": {"p": row(10)}})
        self.assertEqual(len(problems), 1)
        self.assertIn("drawn 9, baseline 10", problems[0])

    def test_a_predicate_missing_from_the_baseline_fails_on_an_exact_run(self):
        problems = compare({"c1": {"p": row(10, 1, 1), "q": row(10)}, "c2": {"p": row(10)}})
        self.assertEqual(problems, ["c1 q: not in the baseline"])

    def test_a_predicate_the_run_did_not_draw_fails(self):
        problems = compare({"c1": {"p": row(10, 1, 1)}, "c2": {}})
        self.assertEqual(problems, ["c2 p: in the baseline but not drawn by this run"])

    def test_a_predicate_the_run_did_not_draw_fails_under_every_seed(self):
        undrawn = "c2 p: in the baseline but not drawn by this run"
        summary = {"c1": {"p": row(10, 1, 1)}, "c2": {}}
        self.assertEqual(compare(summary, seed=2), [undrawn])
        self.assertEqual(compare(summary, examples=50), [undrawn])
        self.assertEqual(compare({"c2": {}}, full=False), [undrawn])
        self.assertEqual(compare({"c2": {}}, full=False, seed=2), [undrawn])

    def test_a_configuration_left_out_fails_only_on_a_full_run(self):
        summary = {"c1": {"p": row(10, 1, 1)}}
        self.assertEqual(compare(summary, full=True), ["c2: in the baseline but not run"])
        self.assertEqual(compare(summary, full=False), [])

    def test_a_subset_with_the_baseline_seed_still_compares_counts(self):
        problems = compare({"c1": {"p": row(10, 2, 1)}}, full=False)
        self.assertEqual(len(problems), 1)
        self.assertIn(f"{LIB_ONLY} 2, baseline 1", problems[0])

    def test_another_seed_fails_only_on_a_new_direction(self):
        base = baseline({"c1": {"p": {"drawn": 10, LIB_ONLY: 1, MODEL_ONLY: 0}}})
        self.assertEqual(compare({"c1": {"p": row(7, 3, 0)}}, base, seed=2), [])
        problems = compare({"c1": {"p": row(7, 3, 1)}}, base, seed=2)
        self.assertEqual(problems, [f"c1 p: {MODEL_ONLY} 1, none in the baseline"])

    def test_another_example_count_is_not_exact_either(self):
        self.assertEqual(compare({"c1": {"p": row(50, 4, 6)}, "c2": {"p": row(50)}}, examples=50), [])

    def test_a_disagreement_without_a_tracked_by_row_fails(self):
        base = baseline(BASE["counts"], tracked={})
        problems = compare({"c1": {"p": row(10, 1, 1)}, "c2": {"p": row(10)}}, base)
        self.assertEqual(problems, ["c1 p: disagreements with no task row in tracked_by"])


class PartialGateNoteTest(unittest.TestCase):
    def note(self, seed=1, examples=10, full=True):
        return behavior_diff.partial_gate_note(BASE, seed, examples, full)

    def test_the_whole_gate_says_nothing(self):
        self.assertIsNone(self.note())

    def test_a_subset_with_the_baseline_seed_says_it_compares_the_configurations_it_ran(self):
        self.assertIn("covers only some configurations", self.note(full=False))
        self.assertIn("compared with the baseline", self.note(full=False))

    def test_another_seed_or_example_count_says_counts_are_not_compared(self):
        for kwargs in ({"seed": 2}, {"examples": 11}, {"seed": 2, "full": False}, {"examples": 11, "full": False}):
            with self.subTest(**kwargs):
                self.assertIn("counts are not compared", self.note(**kwargs))


class BaselineShapeTest(unittest.TestCase):
    def check(self, mutate, update=False):
        base = copy.deepcopy(BASE)
        mutate(base)
        behavior_diff.check_baseline_shape(base, "baseline.json", update)

    def test_the_test_baseline_is_well_formed(self):
        self.check(lambda b: None)

    def test_malformed_baselines_are_harness_errors(self):
        cases = {
            "a direction missing": lambda b: b["counts"]["c1"]["p"].pop(MODEL_ONLY),
            "a count that is a bool": lambda b: b["counts"]["c1"]["p"].update({LIB_ONLY: True}),
            "a negative count": lambda b: b["counts"]["c1"]["p"].update({LIB_ONLY: -1}),
            "directions above drawn": lambda b: b["counts"]["c1"]["p"].update({"drawn": 1}),
            "no counts": lambda b: b.update({"counts": {}}),
            "a note that is not a string": lambda b: b["tracked_by"].update({"p": 3}),
            "no seed": lambda b: b.pop("seed"),
        }
        for name, mutate in cases.items():
            with self.subTest(name), self.assertRaises(behavior_diff.HarnessError):
                self.check(mutate)

    def test_update_may_start_from_a_file_holding_only_tracked_by(self):
        behavior_diff.check_baseline_shape({"tracked_by": {"p": "TB-1"}}, "baseline.json", True)
        with self.assertRaises(behavior_diff.HarnessError):
            behavior_diff.check_baseline_shape({"tracked_by": {"p": "TB-1"}}, "baseline.json", False)


class DigitCountsTest(unittest.TestCase):
    def test_digits_as_written(self):
        # (total, fractional, integer)
        cases = {"0": (0, 0, 0), "0E+1": (0, 0, 0), "0.00": (2, 2, 0), "1E+4": (5, 0, 5), "-1.50": (3, 2, 1),
                 "123.4": (4, 1, 3), "0.5": (1, 1, 0)}
        for text, expected in cases.items():
            with self.subTest(text):
                self.assertEqual(behavior_diff.digit_counts(Decimal(text)), expected)


if __name__ == "__main__":
    unittest.main()
