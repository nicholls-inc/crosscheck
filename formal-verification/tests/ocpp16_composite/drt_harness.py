#!/usr/bin/env python3
"""Differential random testing of EVerest's OCPP 1.6 composite schedule
against the Lean model CrosscheckModel.Ocpp16Composite.

Each case is checked at every instant where either side may change value,
and one second either side of it. The conformance rule is Q4 of the signed
informal spec: EVerest's limit must be at or below the model's. Above by
more than 0.05 of the unit is a fail, below by more than 0.05 is a warn.
"""
from __future__ import annotations

import argparse
import copy
import datetime as dt
import json
import os
import random
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
LEAN_DIR = REPO / "formal-verification" / "lean"
MODEL = LEAN_DIR / "CrosscheckModel" / "Ocpp16Composite.lean"
RUNNER = LEAN_DIR / "CrosscheckModel" / "Ocpp16CompositeRunner.lean"
SUT = HERE / "everest_driver" / "build" / "everest_composite"
BUILD = HERE / ".lean-build"
TOOLCHAIN = "leanprover/lean4:v4.28.0"
ELAN = os.environ.get("ELAN", str(Path.home() / ".elan" / "bin" / "elan"))

DAY = 86400
WEEK = 7 * DAY
TOLERANCE = 0.05
MUTATE_STACK = False
LEAP_CORRECT = False
# Instants just after each leap second inserted since 2010 (IERS Bulletin C).
LEAP_SECONDS = [1341100800, 1435708800, 1483228800]
PURPOSE_OCPP = {"chargePointMax": "ChargePointMaxProfile", "txDefault": "TxDefaultProfile", "tx": "TxProfile"}
KIND_OCPP = {"absolute": ("Absolute", None), "daily": ("Recurring", "Daily"), "weekly": ("Recurring", "Weekly")}


def rfc3339(t: int) -> str:
    return dt.datetime.fromtimestamp(t, dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def epoch(y: int, mo: int, d: int) -> int:
    return int(dt.datetime(y, mo, d, tzinfo=dt.timezone.utc).timestamp())


def driver_tz_dir() -> Path:
    """The directory date 3.0.1's discover_tz_dir() picks for the driver
    (everest_driver/.deps/date-3.0.1/src/tz.cpp, built with USE_OS_TZDB).
    It ignores TZDIR."""
    if sys.platform == "darwin":
        try:
            target = os.readlink("/etc/localtime")
        except OSError as e:
            sys.exit(f"cannot find the driver's tz directory: /etc/localtime is not a readable symlink ({e})")
        i = target.find("zoneinfo")
        j = target.find("/", i) if i >= 0 else -1
        if j < 0:
            sys.exit(f"cannot find the driver's tz directory: /etc/localtime -> {target}")
        return Path(target[:j])
    uclibc = Path("/usr/share/zoneinfo/uclibc")
    return uclibc if uclibc.is_dir() else Path("/usr/share/zoneinfo")


def check_host_leap_seconds() -> None:
    """The driver is built with USE_OS_TZDB, so EVerest counts the leap seconds
    in the host's tz database. Refuse to run if they differ from LEAP_SECONDS,
    because the old-anchor results and the leap attribution depend on them."""
    path = driver_tz_dir() / "leapseconds"
    if not path.exists():
        sys.exit(f"{path} not found. date falls back to leap-seconds.list there, "
                 "which this check does not read, so the leap seconds EVerest counts cannot be confirmed")
    months = {m: n for n, m in enumerate(
        ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"], 1)}
    host = []
    for line in path.read_text().splitlines():
        f = line.split()
        if f and f[0] == "Leap":
            # date counts every Leap line as an inserted second and reads only its
            # date, so refuse any Leap line this check would read differently.
            if len(f) < 7 or f[5] != "+" or f[2] not in months:
                sys.exit(f"{path}: cannot check this Leap line: {line.strip()}")
            after = epoch(int(f[1]), months[f[2]], int(f[3])) + DAY
            if after > epoch(2010, 1, 1):
                host.append(after)
    if host != LEAP_SECONDS:
        sys.exit(f"host leap seconds since 2010 {host} differ from LEAP_SECONDS {LEAP_SECONDS} ({path})")


# ---------------------------------------------------------------- generation

def gen_case(rng: random.Random, old_anchors: bool) -> dict:
    start = epoch(2026, 1, 1) + rng.randrange(0, 300) * DAY + rng.randrange(0, 24) * 3600 + rng.choice([0, 0, 900, 1800])
    length = rng.choice([DAY, 2 * DAY])
    unit = rng.choice(["A", "A", "W"])
    tx_default_connector = rng.choice([0, 1])
    profiles = []
    for purpose, (lo, hi) in (("chargePointMax", (0, 2)), ("txDefault", (0, 3)), ("tx", (0, 2))):
        levels = rng.sample(range(6), rng.randint(lo, hi))
        for level in levels:
            connector = {"chargePointMax": 0, "tx": 1, "txDefault": tx_default_connector}[purpose]
            profiles.append(gen_profile(rng, purpose, connector, level, start, unit, old_anchors))
    if not profiles:
        profiles.append(gen_profile(rng, "txDefault", tx_default_connector, 0, start, unit, old_anchors))
    return {"start": start, "length": length, "unit": unit, "connector": 1, "profiles": profiles}


def gen_profile(rng, purpose, connector, level, start, unit, old_anchors) -> dict:
    kind = rng.choices(["absolute", "daily", "weekly"], weights=[50, 35, 15])[0]
    if kind == "absolute":
        anchor = start + rng.randrange(-12 * 3600, 30 * 3600, 60)
    else:
        years = (2010, 2016) if old_anchors else (2017, 2025)
        anchor = epoch(rng.randint(*years), rng.randint(1, 12), rng.randint(1, 28)) + rng.randrange(0, DAY, 900)
    duration = rng.choice([None, None, 900, 3600, 4 * 3600, 12 * 3600, DAY, 2 * DAY])
    span = duration or DAY
    n_periods = rng.randint(1, 4)
    offsets = sorted(rng.sample(range(900, span, 900), min(n_periods - 1, len(range(900, span, 900))))) if span > 900 else []
    top = 320 if unit == "A" else 220000
    periods = [[0, rng.randint(0, top)]] + [[o, rng.randint(0, top)] for o in offsets]
    valid_from = start + rng.randrange(-6 * 3600, 20 * 3600, 60) if rng.random() < 0.3 else None
    valid_to = None
    if rng.random() < 0.3:
        base = valid_from if valid_from is not None else start
        valid_to = base + rng.randrange(3600, 30 * 3600, 60)
    return {"connectorId": connector, "stackLevel": level, "purpose": purpose, "kind": kind,
            "startSchedule": anchor, "duration": duration, "validFrom": valid_from,
            "validTo": valid_to, "periods": periods}


# ------------------------------------------------------------- both arms

def sut_anchor(p: dict) -> int:
    """With --leap-correct, moves a Recurring anchor forward one second per
    leap second since it, which cancels EVerest's drift if drift is the cause."""
    if LEAP_CORRECT and p["kind"] != "absolute":
        return p["startSchedule"] + sum(1 for leap in LEAP_SECONDS if p["startSchedule"] < leap)
    return p["startSchedule"]


def to_driver_input(case: dict) -> dict:
    out = []
    for i, p in enumerate(case["profiles"]):
        kind, recurrency = KIND_OCPP[p["kind"]]
        level = 9 - p["stackLevel"] if MUTATE_STACK else p["stackLevel"]
        prof = {"chargingProfileId": i + 1, "stackLevel": level,
                "chargingProfilePurpose": PURPOSE_OCPP[p["purpose"]], "chargingProfileKind": kind,
                "chargingSchedule": {"startSchedule": rfc3339(sut_anchor(p)), "chargingRateUnit": case["unit"],
                                     "chargingSchedulePeriod": [{"startPeriod": s, "limit": lim / 10} for s, lim in p["periods"]]}}
        if recurrency:
            prof["recurrencyKind"] = recurrency
        if p["duration"] is not None:
            prof["chargingSchedule"]["duration"] = p["duration"]
        if p["validFrom"] is not None:
            prof["validFrom"] = rfc3339(p["validFrom"])
        if p["validTo"] is not None:
            prof["validTo"] = rfc3339(p["validTo"])
        out.append({"connectorId": p["connectorId"], "profile": prof})
    return {"start": rfc3339(case["start"]), "end": rfc3339(case["start"] + case["length"]),
            "connector": case["connector"], "unit": case["unit"], "session_start": rfc3339(case["start"]),
            "profiles": out}


def run_sut(case: dict) -> tuple[list | None, str]:
    proc = subprocess.run([str(SUT)], input=json.dumps(to_driver_input(case)), capture_output=True, text=True)
    if proc.returncode != 0:
        return None, (proc.stdout + proc.stderr).strip()
    return json.loads(proc.stdout)["intermediate"], ""


def sut_value(intermediate: list, case: dict, t: int) -> float | None:
    offset = t - case["start"]
    key = "current_limit" if case["unit"] == "A" else "power_limit"
    value = None
    for entry in intermediate:
        if entry["startPeriod"] <= offset:
            value = entry[key]
    return None if value is None or value < 0 else value


def query_times(case: dict, intermediate: list | None) -> list[int]:
    lo, hi = case["start"], case["start"] + case["length"]
    points = {lo}
    for p in case["profiles"]:
        starts = [p["startSchedule"]]
        if p["kind"] != "absolute":
            r = DAY if p["kind"] == "daily" else WEEK
            k = max(0, (lo - p["startSchedule"]) // r)
            starts = [p["startSchedule"] + (k + j) * r for j in range(0, (hi - lo) // r + 2)]
        for s in starts:
            points.add(s)
            points.update(s + off for off, _ in p["periods"])
            if p["duration"] is not None:
                points.add(s + p["duration"])
        for v in (p["validFrom"], p["validTo"]):
            if v is not None:
                points.add(v)
    for entry in intermediate or []:
        points.add(lo + entry["startPeriod"])
    times = set()
    for b in points:
        for t in (b - 1, b, b + 1):
            if lo <= t < hi:
                times.add(t)
    return sorted(times)


def ensure_oracle() -> None:
    target = BUILD / "CrosscheckModel" / "Ocpp16Composite.olean"
    if target.exists() and target.stat().st_mtime >= MODEL.stat().st_mtime:
        return
    target.parent.mkdir(parents=True, exist_ok=True)
    proc = subprocess.run([ELAN, "run", TOOLCHAIN, "lean", "-R", str(LEAN_DIR), "-o", str(target), str(MODEL)],
                          capture_output=True, text=True)
    if proc.returncode != 0:
        sys.exit(f"model build failed:\n{proc.stdout}{proc.stderr}")


def run_oracle(cases: list[dict], times: list[list[int]]) -> list[list[int | None]]:
    libdir = subprocess.run([ELAN, "run", TOOLCHAIN, "lean", "--print-libdir"], capture_output=True, text=True).stdout.strip()
    payload = {"cases": [{"connector": c["connector"], "times": ts, "profiles": c["profiles"]} for c, ts in zip(cases, times)]}
    env = dict(os.environ, LEAN_PATH=f"{BUILD}:{libdir}")
    proc = subprocess.run([ELAN, "run", TOOLCHAIN, "lean", "--run", str(RUNNER)], input=json.dumps(payload),
                          capture_output=True, text=True, env=env, cwd=LEAN_DIR)
    if proc.returncode != 0:
        sys.exit(f"oracle failed:\n{proc.stdout}{proc.stderr}")
    return json.loads(proc.stdout)["results"]


# ------------------------------------------------------------- comparison

@dataclass
class Outcome:
    status: str
    t: int | None = None
    model: float | None = None
    sut: float | None = None
    detail: str = ""


def compare(case: dict, intermediate: list | None, err: str, ts: list[int], model: list) -> Outcome:
    if intermediate is None:
        return Outcome("fail", detail=f"SUT error: {err}")
    worst = Outcome("pass")
    for t, m in zip(ts, model):
        if m is None:
            continue
        mv = m / 10
        sv = sut_value(intermediate, case, t)
        if sv is None or sv > mv + TOLERANCE:
            return Outcome("fail", t, mv, sv)
        if sv < mv - TOLERANCE and worst.status == "pass":
            worst = Outcome("warn", t, mv, sv)
    return worst


def check(cases: list[dict]) -> list[Outcome]:
    sut = [run_sut(c) for c in cases]
    times = [query_times(c, inter) for c, (inter, _) in zip(cases, sut)]
    model = run_oracle(cases, times)
    return [compare(c, inter, err, ts, m) for c, (inter, err), ts, m in zip(cases, sut, times, model)]


# ---------------------------------------------------------------- shrinking

def shrink(case: dict, status: str) -> dict:
    def still(c):
        return check([c])[0].status == status

    current = copy.deepcopy(case)
    changed = True
    while changed:
        changed = False
        candidates = []
        for i in range(len(current["profiles"])):
            c = copy.deepcopy(current)
            del c["profiles"][i]
            if c["profiles"]:
                candidates.append(c)
        for i, p in enumerate(current["profiles"]):
            for j in range(1, len(p["periods"])):
                c = copy.deepcopy(current)
                del c["profiles"][i]["periods"][j]
                candidates.append(c)
            for key in ("validFrom", "validTo", "duration"):
                if p[key] is not None:
                    c = copy.deepcopy(current)
                    c["profiles"][i][key] = None
                    candidates.append(c)
        for c in candidates:
            if still(c):
                current, changed = c, True
                break
    return current


# ------------------------------------------------------------------ report

def describe(case: dict, o: Outcome) -> str:
    when = f"{rfc3339(o.t)} (offset {o.t - case['start']} s)" if o.t is not None else "n/a"
    return f"at {when}: model {o.model} {case['unit']}, EVerest {o.sut} {case['unit']} {o.detail}".rstrip()


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--count", type=int, default=2000)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--batch", type=int, default=200)
    ap.add_argument("--old-anchors", action="store_true", help="anchor Recurring profiles in 2010-2016, across leap seconds")
    ap.add_argument("--max-shrink", type=int, default=12)
    ap.add_argument("--report", type=Path, default=HERE / "drt_report_latest.md",
                    help="generated report (default is gitignored; drt_report.md is the curated record)")
    ap.add_argument("--replay", type=int, help="re-run one case index from the seed and print it")
    ap.add_argument("--mutate-stack", action="store_true",
                    help="self-test: reverse stack levels in the SUT's input, so a correct harness must report fails")
    ap.add_argument("--leap-correct", action="store_true",
                    help="attribution check: shift Recurring anchors in the SUT's input by the leap seconds since them")
    args = ap.parse_args()
    global MUTATE_STACK, LEAP_CORRECT
    MUTATE_STACK = args.mutate_stack
    LEAP_CORRECT = args.leap_correct

    if not SUT.exists():
        sys.exit(f"SUT not built: run {HERE / 'everest_driver' / 'build.sh'}")
    check_host_leap_seconds()
    ensure_oracle()

    rng = random.Random(args.seed)
    cases = [gen_case(rng, args.old_anchors) for _ in range(args.count)]
    if args.replay is not None:
        c = cases[args.replay]
        print(json.dumps({"case": c, "driver_input": to_driver_input(c)}, indent=2))
        print(describe(c, check([c])[0]))
        return 0

    outcomes: list[Outcome] = []
    for i in range(0, len(cases), args.batch):
        outcomes += check(cases[i:i + args.batch])

    counts = {s: sum(o.status == s for o in outcomes) for s in ("pass", "warn", "fail")}
    flagged = [(i, o) for i, o in enumerate(outcomes) if o.status != "pass"]
    lines = [
        "# DRT report: Ocpp16Composite", "",
        f"Run: {dt.datetime.now(dt.timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')}",
        f"RNG seed: {args.seed}", f"Cases: {args.count}", "D4 case: (a)",
        f"Recurring anchors: {'2010-2016 (across leap seconds)' if args.old_anchors else '2017-2025 (no leap seconds since)'}",
        f"SUT input altered: {', '.join(f for f, on in (('--mutate-stack', args.mutate_stack), ('--leap-correct', args.leap_correct)) if on) or 'no'}",
        "SUT: EVerest lib/everest/ocpp/lib/ocpp/v16/profile.cpp at dd696165, via everest_driver",
        "Oracle: CrosscheckModel.Ocpp16Composite via Ocpp16CompositeRunner.lean", "",
        "## Verdict summary", "", "| Status | Cases |", "| --- | --- |",
        f"| Pass | {counts['pass']} |", f"| Warn (EVerest below the model) | {counts['warn']} |",
        f"| Fail (EVerest above the model, or no limit, or error) | {counts['fail']} |", "",
        "## Flagged cases, minimised", "",
    ]
    for n, (i, o) in enumerate(flagged[:args.max_shrink]):
        small = shrink(cases[i], o.status)
        so = check([small])[0]
        lines += [f"### Case {i}: {o.status}", "",
                  f"- Original: {describe(cases[i], o)}",
                  f"- Minimised: {describe(small, so)}",
                  f"- Replay: `python3 {Path(__file__).relative_to(REPO)} --seed {args.seed} --count {args.count}"
                  f"{' --old-anchors' if args.old_anchors else ''}{' --mutate-stack' if args.mutate_stack else ''}"
                  f" --replay {i}`", "",
                  "```json", json.dumps(small, indent=1), "```", ""]
    if len(flagged) > args.max_shrink:
        lines.append(f"{len(flagged) - args.max_shrink} more flagged cases not minimised. Indices: "
                     + ", ".join(str(i) for i, _ in flagged[args.max_shrink:]))
    args.report.write_text("\n".join(lines) + "\n")
    print(f"pass {counts['pass']}  warn {counts['warn']}  fail {counts['fail']}  report {args.report}")
    return 1 if counts["fail"] else 0


if __name__ == "__main__":
    sys.exit(main())
