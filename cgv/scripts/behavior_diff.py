#!/usr/bin/env python3
"""Compare the predicates of BehaviorModel.lean with Django and pydantic (TB-1.14).

Hypothesis draws inputs for each predicate from a fixed seed. The real Django
model field's clean() or pydantic validation gives one verdict, and the
predicate, evaluated by Lean from the text of BehaviorModel.lean
(prover/scripts/BehaviorEval.lean), gives the other. The report counts the
inputs and the disagreements per configuration and predicate.

Subcommands (run from cgv/):
  run       Build the Lean evaluator and the pinned virtual environments, sample
            every configuration, evaluate, compare with scripts/behavior-baseline.json.
            Exit 0 when every count matches the baseline, 1 when one differs (or, with
            --update-baseline, a disagreeing predicate has no tracked_by row), 2 when the
            harness itself fails.
  selftest  Mutate charFieldAccepts (<= becomes <) in a scratch copy of
            BehaviorModel.lean, run against it, and exit 0 only if the run
            fails with new charFieldAccepts disagreements and nothing else moves.
  sample    Internal. Draw inputs and run one library configuration inside its
            virtual environment.

  python3 scripts/behavior_diff.py run
  python3 scripts/behavior_diff.py run --seed 7 --examples 200
  python3 scripts/behavior_diff.py selftest

Needs python3.12 (or --python), the Lean toolchain named in prover/lean-toolchain
through elan, and network access to the package index on the first run.
"""

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
from collections import defaultdict
from decimal import Decimal
from pathlib import Path

CGV = Path(__file__).resolve().parent.parent
DEFAULT_SEED = 20261021
DEFAULT_EXAMPLES = 1000
EXAMPLES_SHOWN = 3

ENVIRONMENTS = {
    "django42": CGV / "scripts" / "behavior-requirements-django42.txt",
    "django52": CGV / "scripts" / "behavior-requirements-django52.txt",
}

DEFAULT_MODEL = CGV / "prover" / "ContractGraph" / "BehaviorModel.lean"

CONFIGS = {
    "django42-sqlite": ("django42", "django", "sqlite"),
    "django42-base": ("django42", "django", "base"),
    "django52-sqlite": ("django52", "django", "sqlite"),
    "django52-base": ("django52", "django", "base"),
    "pydantic": ("django52", "pydantic", None),
}

MUTATION = (
    "def charFieldAccepts (maxLength : Nat) (actualLength : Nat) : Prop :=\n  actualLength ≤ maxLength",
    "def charFieldAccepts (maxLength : Nat) (actualLength : Nat) : Prop :=\n  actualLength < maxLength",
)


class HarnessError(Exception):
    pass


def digit_counts(v):
    """(total, fractional, integer) digits of a finite Decimal as written.

    Fractional digits are the places after the point, trailing zeros included.
    Integer digits are the digits of the integer part, so zero has none.
    """
    exponent = v.as_tuple().exponent
    fractional = max(0, -exponent)
    whole = int(abs(v))
    integer = len(str(whole)) if whole else 0
    return integer + fractional, fractional, integer


def sample(config, seed, examples, out):
    _, library, backend = CONFIGS[config]
    if library == "django":
        probes = django_probes(backend)
    else:
        probes = pydantic_probes()
    import hypothesis
    from hypothesis import HealthCheck, Phase, given, settings

    with open(out, "w") as fh:
        for name, strategy, verdict in probes:
            drawn = []

            @settings(max_examples=examples, database=None, deadline=None, phases=[Phase.generate],
                      suppress_health_check=list(HealthCheck))
            @hypothesis.seed(seed)
            @given(strategy)
            def draw(case):
                drawn.append(case)

            draw()
            for case in drawn:
                accepted, args, shown = verdict(case)
                fh.write(json.dumps({"config": config, "predicate": name, "input": shown,
                                     "args": args, "library": accepted}) + "\n")
    import django
    import pydantic
    versions = {"python": sys.version.split()[0], "django": django.__version__,
                "pydantic": pydantic.VERSION, "hypothesis": hypothesis.__version__}
    Path(str(out) + ".versions").write_text(json.dumps(versions))


def decimals(st, max_places=10, max_exponent=4):
    @st.composite
    def build(draw):
        sign = draw(st.integers(0, 1))
        coefficient = draw(st.one_of(st.integers(0, 10**10), st.integers(0, 1000)))
        exponent = draw(st.integers(-max_places, max_exponent))
        return Decimal((sign, tuple(int(c) for c in str(coefficient)), exponent))

    return build()


def repeated(st, strategy):
    """strategy, drawn as often as any other branch: Hypothesis skips a choice sequence it has already tried,
    so st.none() alone yields None about once per run."""
    return st.tuples(strategy, st.integers(0, 10**6)).map(lambda t: t[0])


def near(st, lo, hi):
    """Pairs (limit, value) in [lo, hi], half of them within 3 of each other."""

    @st.composite
    def build(draw):
        limit = draw(st.integers(lo, hi))
        if draw(st.booleans()):
            value = min(hi, max(lo, limit + draw(st.integers(-3, 3))))
        else:
            value = draw(st.integers(lo, hi))
        return limit, value

    return build()


def micros_pair(st):
    """(limit, value) Decimals with 0 to 6 places, half of them drawn within 3 micros before rounding."""
    @st.composite
    def build(draw):
        limit_micros, value_micros = draw(near(st, -10**15, 10**15))

        def as_decimal(micros):
            places = draw(st.integers(0, 6))
            micros -= micros % 10 ** (6 - places)
            return Decimal(micros).scaleb(-6).quantize(Decimal(1).scaleb(-places))

        return as_decimal(limit_micros), as_decimal(value_micros)

    return build()


def micros(d):
    scaled = d.scaleb(6)
    if scaled != scaled.to_integral_value():
        raise HarnessError(f"{d} has more than 6 decimal places")
    return int(scaled)


def django_probes(backend):
    import django
    from django.conf import settings as dj_settings

    engine = {"sqlite": "django.db.backends.sqlite3", "base": "django.db.backends.dummy"}[backend]
    dj_settings.configure(DATABASES={"default": {"ENGINE": engine, "NAME": ":memory:"}}, USE_TZ=True)
    django.setup()
    from django.core.exceptions import ValidationError
    from django.core.validators import MaxValueValidator, MinValueValidator
    from django.db import models
    from hypothesis import strategies as st

    def accepts(field, value):
        try:
            field.clean(value, None)
            return True
        except ValidationError:
            return False

    def decimal_field(case):
        (m, d), v = case
        t, f, i = digit_counts(v)
        field = models.DecimalField(max_digits=m, decimal_places=d, blank=True)
        return accepts(field, v), [m, d, t, f, i], {"max_digits": m, "decimal_places": d, "value": str(v)}

    def null_field(null):
        def verdict(v):
            field = models.IntegerField(null=null, blank=True)
            return accepts(field, v), [v is None], {"null": null, "value": v}
        return verdict

    def char_field(case):
        n, s = case
        field = models.CharField(max_length=n, blank=True)
        return accepts(field, s), [n, len(s)], {"max_length": n, "value": s}

    def positive_field(v):
        return accepts(models.PositiveIntegerField(blank=True), v), [v], {"value": v}

    def value_validator(validator):
        def verdict(case):
            limit, v = case
            field = models.IntegerField(blank=True, validators=[validator(limit)])
            return accepts(field, v), [limit, v], {"limit": limit, "value": v}
        return verdict

    def choices_field(case):
        choices, value, blank = case
        field = models.CharField(max_length=100, choices=[(c, c) for c in choices], blank=blank)
        return accepts(field, value), [choices, value], {"choices": choices, "value": value, "blank": blank}

    def micros_validator(validator):
        def verdict(case):
            limit, v = case
            field = models.DecimalField(max_digits=30, decimal_places=6, blank=True,
                                        validators=[validator(limit)])
            return accepts(field, v), [micros(limit), micros(v)], {"limit": str(limit), "value": str(v)}
        return verdict

    digits = st.integers(1, 8).flatmap(lambda m: st.tuples(st.just(m), st.integers(0, m)))
    ints = st.one_of(st.integers(-2**70, 2**70), st.integers(-5, 5),
                     st.sampled_from([2**31 - 1, 2**31, 2**63 - 1, 2**63, -2**63, -2**63 - 1]))
    small = st.one_of(repeated(st, st.none()), st.integers(-1000, 1000))
    choice_text = st.text(alphabet="abé ", max_size=2)

    @st.composite
    def choices_case(draw):
        choices = draw(st.lists(choice_text, max_size=4, unique=True))
        pool = st.one_of(choice_text, st.sampled_from(choices)) if choices else choice_text
        return choices, draw(pool), draw(st.booleans())

    int32 = (-2**31, 2**31 - 1)
    return [
        ("decimalFieldAccepts", st.tuples(digits, decimals(st)), decimal_field),
        ("notNullAccepts", small, null_field(False)),
        ("nullableAccepts", small, null_field(True)),
        ("charFieldAccepts", st.tuples(st.integers(0, 20), st.text(max_size=30)), char_field),
        ("positiveIntFieldAccepts", ints, positive_field),
        ("minValueAccepts", near(st, *int32), value_validator(MinValueValidator)),
        ("maxValueAccepts", near(st, *int32), value_validator(MaxValueValidator)),
        ("choicesAccepts", choices_case(), choices_field),
        ("maxValueAcceptsMicros", micros_pair(st), micros_validator(MaxValueValidator)),
        ("minValueAcceptsMicros", micros_pair(st), micros_validator(MinValueValidator)),
    ]


def pydantic_probes():
    from typing import Optional, Union

    from hypothesis import strategies as st
    from pydantic import ConfigDict, Field, ValidationError, constr, create_model

    models = {}

    def model(key, annotation, field=None, strict=False):
        key = repr(key)
        if key not in models:
            config = ConfigDict(strict=True) if strict else None
            models[key] = create_model("M", __config__=config, x=(annotation, ... if field is None else field))
        return models[key]

    def accepts(m, value):
        try:
            m(x=value)
            return True
        except ValidationError:
            return False

    def decimal_field(case):
        (m, d), v = case
        t, f, i = digit_counts(v)
        return (accepts(model(("decimal", m, d), Decimal, Field(max_digits=m, decimal_places=d)), v), [m, d, t, f, i],
                {"max_digits": m, "decimal_places": d, "value": str(v)})

    def max_length(case):
        n, form, s = case
        if form == "Field":
            m = model(("Field", n), str, Field(max_length=n))
        else:
            m = model(("constr", n), constr(max_length=n))
        return accepts(m, s), [n, len(s)], {"max_length": n, "form": form, "value": s}

    def le(case):
        (limit, v), form = case
        if form == "le":
            return accepts(model(("le", limit), int, Field(le=limit)), v), [limit, v], {"le": limit, "value": v}
        return accepts(model(("lt", limit), int, Field(lt=limit)), v), [limit - 1, v], {"lt": limit, "value": v}

    optional = {"Optional[int]": Optional[int], "int | None": int | None, "Union[int, None]": Union[int, None]}

    def annotation_null(case):
        spelling, v = case
        annotation = int if spelling == "int" else optional[spelling]
        return (accepts(model(("null", spelling), annotation), v), [spelling != "int", v is None],
                {"annotation": spelling, "value": v})

    numeric_types = {"int": int, "float": float, "str": str, "bool": bool, "Decimal": Decimal}

    def numeric(case):
        target, v = case
        shown = str(v) if isinstance(v, (Decimal, float)) else v
        return (accepts(model(("strict", target), numeric_types[target], strict=True), v), [type(v).__name__, target],
                {"annotation": target, "type": type(v).__name__, "value": shown})

    def micros_bound(keyword):
        def verdict(case):
            limit, v = case
            m = model(("micros", keyword, str(limit)), Decimal, Field(**{keyword: limit}))
            return accepts(m, v), [micros(limit), micros(v)], {keyword: str(limit), "value": str(v)}
        return verdict

    digits = st.integers(1, 8).flatmap(lambda m: st.tuples(st.just(m), st.integers(0, m)))
    values = st.one_of(st.integers(-2**70, 2**70), st.floats(), st.text(max_size=3), repeated(st, st.booleans()),
                       decimals(st))
    return [
        ("pydanticDecimalAccepts", st.tuples(digits, decimals(st)), decimal_field),
        ("pydanticMaxLengthAccepts",
         st.tuples(st.integers(0, 20), st.sampled_from(["Field", "constr"]), st.text(max_size=30)), max_length),
        ("pydanticLeAccepts", st.tuples(near(st, -2**70, 2**70), st.sampled_from(["le", "lt"])), le),
        ("annotationAcceptsNull",
         st.tuples(st.sampled_from(["int", *optional]), st.one_of(repeated(st, st.none()), st.integers(-1000, 1000))),
         annotation_null),
        ("annotationAcceptsNumeric", st.tuples(st.sampled_from(list(numeric_types)), values), numeric),
        ("maxValueAcceptsMicros", micros_pair(st), micros_bound("le")),
        ("minValueAcceptsMicros", micros_pair(st), micros_bound("ge")),
    ]


def lean_command():
    proxy = Path(os.environ.get("ELAN_HOME") or Path.home() / ".elan") / "bin" / "lean"
    found = str(proxy) if proxy.exists() else shutil.which("lean")
    if not found or not Path(found).exists():
        raise HarnessError("lean not found: install elan, https://github.com/leanprover/elan")
    toolchain = (CGV / "prover" / "lean-toolchain").read_text().strip()
    return [found, f"+{toolchain}"]


def build_model(model_file, work):
    src = work / "model-src" / "ContractGraph"
    olean = work / "model-olean" / "ContractGraph"
    for d in (src, olean):
        shutil.rmtree(d.parent, ignore_errors=True)
        d.mkdir(parents=True)
    shutil.copyfile(model_file, src / "BehaviorModel.lean")
    result = subprocess.run([*lean_command(), "-R", str(src.parent), "-o", str(olean / "BehaviorModel.olean"),
                             str(src / "BehaviorModel.lean")], capture_output=True, text=True)
    if result.returncode != 0:
        raise HarnessError(f"lean could not build {model_file}:\n{result.stdout}{result.stderr}")
    return olean.parent


def evaluate(cases, olean_root):
    lines = "".join(json.dumps({"p": c["predicate"], "a": c["args"]}) + "\n" for c in cases)
    env = dict(os.environ, LEAN_PATH=str(olean_root))
    result = subprocess.run([*lean_command(), "--run", str(CGV / "prover" / "scripts" / "BehaviorEval.lean")],
                            input=lines, capture_output=True, text=True, env=env)
    answers = result.stdout.splitlines()
    if result.returncode != 0 or len(answers) != len(cases):
        raise HarnessError(f"the Lean evaluator failed (exit {result.returncode}, {len(answers)} answers "
                           f"for {len(cases)} cases):\n{result.stderr}")
    verdicts = []
    for case, answer in zip(cases, answers):
        if answer not in ("true", "false"):
            raise HarnessError(f"the Lean evaluator answered {answer!r} for {case}")
        verdicts.append(answer == "true")
    return verdicts


def ensure_venv(name, python, index_url, work):
    requirements = ENVIRONMENTS[name]
    interpreter = subprocess.run([python, "-c", "import sys; print(sys.version)"], check=True,
                                 capture_output=True, text=True).stdout
    digest = hashlib.sha256(requirements.read_bytes() + python.encode() + interpreter.encode()).hexdigest()
    venv = work / f"venv-{name}"
    marker = venv / "requirements.sha256"
    if marker.exists() and marker.read_text() == digest:
        return venv / "bin" / "python"
    shutil.rmtree(venv, ignore_errors=True)
    subprocess.run([python, "-m", "venv", str(venv)], check=True)
    subprocess.run([str(venv / "bin" / "python"), "-m", "pip", "install", "--quiet", "--disable-pip-version-check",
                    "--require-hashes", "--no-deps", "--index-url", index_url, "-r", str(requirements)], check=True)
    marker.write_text(digest)
    return venv / "bin" / "python"


def summarise(cases, verdicts):
    groups = defaultdict(list)
    for case, model in zip(cases, verdicts):
        groups[(case["config"], case["predicate"])].append((case, model))
    summary = {}
    for (config, predicate), rows in sorted(groups.items()):
        library_only = [c["input"] for c, model in rows if c["library"] and not model]
        model_only = [c["input"] for c, model in rows if model and not c["library"]]

        def shortest(xs):
            unique = {json.dumps(x, sort_keys=True, ensure_ascii=False): x for x in xs}
            return [unique[k] for k in sorted(unique, key=lambda k: (len(k), k))[:EXAMPLES_SHOWN]]

        summary.setdefault(config, {})[predicate] = {
            "drawn": len(rows),
            "distinct": len({json.dumps(c["input"], sort_keys=True) for c, _ in rows}),
            "library_accepts": sum(c["library"] for c, _ in rows),
            "model_accepts": sum(model for _, model in rows),
            "disagreements": len(library_only) + len(model_only),
            "library_accepts_model_rejects": shortest(library_only),
            "model_accepts_library_rejects": shortest(model_only),
        }
    return summary


def compare(summary, baseline, seed, examples, full=True):
    """Problems that make the run fail, as strings. `full` is true when every configuration ran."""
    problems = []
    exact = baseline.get("seed") == seed and baseline.get("examples") == examples
    tracked = baseline.get("tracked_by", {})
    for config, predicates in baseline.get("counts", {}).items():
        if config not in summary:
            if full:
                problems.append(f"{config}: in the baseline but not run")
            continue
        for predicate in predicates:
            if predicate not in summary[config]:
                problems.append(f"{config} {predicate}: in the baseline but not drawn by this run")
    for config, predicates in summary.items():
        for predicate, row in predicates.items():
            known = baseline.get("counts", {}).get(config, {}).get(predicate)
            if exact:
                if known is None:
                    problems.append(f"{config} {predicate}: not in the baseline")
                elif (known["drawn"], known["disagreements"]) != (row["drawn"], row["disagreements"]):
                    problems.append(f"{config} {predicate}: {row['drawn']} drawn and {row['disagreements']} "
                                    f"disagreements, baseline {known['drawn']} and {known['disagreements']}")
            elif row["disagreements"] and not (known and known["disagreements"]):
                problems.append(f"{config} {predicate}: {row['disagreements']} disagreements, none in the baseline")
            if row["disagreements"] and predicate not in tracked:
                problems.append(f"{config} {predicate}: disagreements with no task row in tracked_by")
    return problems


def print_summary(summary, tracked):
    print(f"{'configuration':<16} {'predicate':<26} {'drawn':>6} {'distinct':>8} {'disagree':>8}  row")
    for config, predicates in summary.items():
        for predicate, row in predicates.items():
            note = tracked.get(predicate, "") if row["disagreements"] else ""
            print(f"{config:<16} {predicate:<26} {row['drawn']:>6} {row['distinct']:>8} "
                  f"{row['disagreements']:>8}  {note}")


def run(args):
    work = Path(args.work_dir).resolve()
    configs = list(dict.fromkeys(args.configs.split(","))) if args.configs else list(CONFIGS)
    for c in configs:
        if c not in CONFIGS:
            raise HarnessError(f"unknown configuration {c}; known: {', '.join(CONFIGS)}")
    baseline_path = Path(args.baseline)
    if args.update_baseline:
        if set(configs) != set(CONFIGS):
            raise HarnessError("--update-baseline needs every configuration; drop --configs")
        if Path(args.behavior_model).resolve() != DEFAULT_MODEL.resolve():
            raise HarnessError("--update-baseline only accepts the default BehaviorModel.lean path")
        if (args.seed, args.examples) != (DEFAULT_SEED, DEFAULT_EXAMPLES):
            raise HarnessError("--update-baseline only accepts the default --seed and --examples")
    if baseline_path.exists():
        baseline = json.loads(baseline_path.read_text())
        check_baseline_shape(baseline, baseline_path)
    elif args.update_baseline:
        baseline = {}
    else:
        raise HarnessError(f"baseline {baseline_path} not found; create it with --update-baseline")
    work.mkdir(parents=True, exist_ok=True)
    olean_root = build_model(Path(args.behavior_model).resolve(), work)
    cases, versions = [], {}
    for config in configs:
        python = ensure_venv(CONFIGS[config][0], args.python, args.index_url, Path(args.venv_dir or work).resolve())
        out = work / f"cases-{config}.jsonl"
        subprocess.run([str(python), "-I", __file__, "sample", "--config", config, "--seed", str(args.seed),
                        "--examples", str(args.examples), "--out", str(out)], check=True)
        cases += [json.loads(line) for line in out.read_text().splitlines()]
        versions[config] = json.loads(Path(str(out) + ".versions").read_text())
    verdicts = evaluate(cases, olean_root)
    summary = summarise(cases, verdicts)
    report = {"seed": args.seed, "examples": args.examples, "behavior_model": str(args.behavior_model),
              "versions": versions, "summary": summary}
    (work / "report.json").write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n")
    print_summary(summary, baseline.get("tracked_by", {}))
    print(f"seed {args.seed}, {args.examples} examples per predicate, {len(cases)} cases; "
          f"report in {work / 'report.json'}")
    if args.update_baseline:
        counts = {config: {p: {"drawn": r["drawn"], "disagreements": r["disagreements"]}
                           for p, r in predicates.items()} for config, predicates in summary.items()}
        baseline = {"seed": args.seed, "examples": args.examples, "versions": versions, "counts": counts,
                    "tracked_by": baseline.get("tracked_by", {})}
        problems = [f"{config} {predicate}: disagreements with no task row in tracked_by"
                    for config, predicates in summary.items() for predicate, row in predicates.items()
                    if row["disagreements"] and predicate not in baseline["tracked_by"]]
        if problems:
            print(f"not writing {baseline_path}: add a tracked_by row for each predicate below "
                  "(a new file may hold only a tracked_by object)")
        else:
            staged = baseline_path.with_suffix(baseline_path.suffix + ".tmp")
            try:
                staged.write_text(json.dumps(baseline, indent=2, ensure_ascii=False) + "\n")
                os.replace(staged, baseline_path)
            finally:
                staged.unlink(missing_ok=True)
            print(f"wrote {baseline_path}")
    else:
        problems = compare(summary, baseline, args.seed, args.examples, full=set(configs) == set(CONFIGS))
    for p in problems:
        print(f"FAIL {p}")
    return 1 if problems else 0


def check_baseline_shape(baseline, path):
    counts = baseline.get("counts", {}) if isinstance(baseline, dict) else None
    ok = isinstance(counts, dict) and isinstance(baseline.get("tracked_by", {}), dict) and all(
        isinstance(rows, dict) and all(isinstance(row, dict) and isinstance(row.get("drawn"), int)
                                       and isinstance(row.get("disagreements"), int) for row in rows.values())
        for rows in counts.values())
    if not ok:
        raise HarnessError(f"baseline {path} is malformed; fix it by hand or delete it and use --update-baseline "
                           "(a new file needs a tracked_by object for the disagreeing predicates)")


def baseline_counts(baseline, config, predicate):
    try:
        return baseline["counts"][config][predicate]["disagreements"]
    except KeyError:
        raise HarnessError(f"baseline has no entry for {config} {predicate}; run --update-baseline") from None


def selftest(args):
    work = Path(args.work_dir).resolve()
    model = Path(args.behavior_model).read_text()
    if model.count(MUTATION[0]) != 1:
        raise HarnessError("the charFieldAccepts text to mutate is not in BehaviorModel.lean exactly once")
    mutant = work / "mutant" / "BehaviorModel.lean"
    mutant.parent.mkdir(parents=True, exist_ok=True)
    mutant.write_text(model.replace(*MUTATION))
    run_args = argparse.Namespace(**{**vars(args), "behavior_model": str(mutant), "work_dir": str(work / "mutant"),
                                     "venv_dir": str(work), "update_baseline": False})
    status = run(run_args)
    report = json.loads((work / "mutant" / "report.json").read_text())
    baseline = json.loads(Path(args.baseline).read_text())
    moved = {(config, predicate) for config, predicates in report["summary"].items()
             for predicate, row in predicates.items()
             if row["disagreements"] != baseline_counts(baseline, config, predicate)}
    django = {(config, "charFieldAccepts") for config in report["summary"] if config.startswith("django")}
    if status == 1 and moved == django:
        print(f"selftest passed: the mutant moved exactly {sorted(moved)}")
        return 0
    print(f"selftest FAILED: exit {status}, changed counts {sorted(moved)}, expected {sorted(django)}")
    return 1


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    for name in ("run", "selftest"):
        p = sub.add_parser(name)
        p.add_argument("--seed", type=int, default=DEFAULT_SEED)
        p.add_argument("--examples", type=int, default=DEFAULT_EXAMPLES, help="per configuration and predicate")
        p.add_argument("--configs", help=f"comma-separated subset of {', '.join(CONFIGS)}")
        p.add_argument("--behavior-model", default=str(DEFAULT_MODEL))
        p.add_argument("--baseline", default=str(CGV / "scripts" / "behavior-baseline.json"))
        p.add_argument("--work-dir", default=str(CGV / "target" / "behavior-diff"))
        p.add_argument("--python", default="python3.12", help="interpreter for the virtual environments")
        p.add_argument("--index-url", default="https://pypi.org/simple")
        p.add_argument("--update-baseline", action="store_true")
        p.set_defaults(venv_dir=None)
    s = sub.add_parser("sample")
    s.add_argument("--config", required=True, choices=list(CONFIGS))
    s.add_argument("--seed", type=int, required=True)
    s.add_argument("--examples", type=int, required=True)
    s.add_argument("--out", required=True)
    args = parser.parse_args()
    try:
        if args.command == "sample":
            sample(args.config, args.seed, args.examples, args.out)
            return 0
        return run(args) if args.command == "run" else selftest(args)
    except (HarnessError, subprocess.CalledProcessError, OSError, UnicodeDecodeError, json.JSONDecodeError) as e:
        print(f"ERROR {e}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
