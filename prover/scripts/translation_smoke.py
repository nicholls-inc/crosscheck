#!/usr/bin/env python3
"""Translation smoke test for the data-flow v2 SQLite interface.

Builds a SQLite database with the v2 schema (docs/design/dataflow-v2.md on top
of the schema in src/db.rs), runs the Lean checker on it and asserts on the
JSON. Covers a per-edge source override, target_param filtering, a range row
with a lower bound, a calls edge (not followed) and hop attribution. Also runs
the checker on a database with the v1 schema (no v2 columns), which must still
translate, and on a round-3 database (edges.site_file/site_line,
contracts.param_min_micros/param_max_micros, JSON param_choices): node
locations, sites, exact bounds, legacy REAL fallback, choices, and the
state budgets (--max-states / --max-paths alias, --max-states-per-edge; exit
code 2), and on a round-5 database (nodes.is_call_site,
contracts.param_min_decimal/param_max_decimal): exact decimal bounds, no
warnings on call-site suffix paths, guarantee_at and states_checked.

Usage: prover/scripts/translation_smoke.py [path/to/contract-graph-checker]
Requires: cd prover && lake build
"""

import json
import os
import sqlite3
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
DEFAULT_CHECKER = os.path.join(HERE, "..", ".lake", "build", "bin", "contract-graph-checker")

SCHEMA_V2 = """
CREATE TABLE nodes (
    id             INTEGER PRIMARY KEY,
    name           TEXT NOT NULL,
    kind           TEXT NOT NULL CHECK (kind IN ('model', 'function', 'field')),
    source_file    TEXT NOT NULL,
    source_line    INTEGER NOT NULL,
    qualified_name TEXT
);

CREATE TABLE edges (
    id              INTEGER PRIMARY KEY,
    source_node_id  INTEGER NOT NULL REFERENCES nodes(id),
    target_node_id  INTEGER NOT NULL REFERENCES nodes(id),
    relationship    TEXT NOT NULL CHECK (relationship IN (
                        'calls', 'writes_to', 'flows_to'
                    )),
    discovery       TEXT NOT NULL CHECK (discovery IN ('ast_pattern', 'manual', 'type_inference')),
    target_param    TEXT,
    source_override INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE contracts (
    id                  INTEGER PRIMARY KEY,
    node_id             INTEGER NOT NULL REFERENCES nodes(id),
    constraint_type     TEXT NOT NULL CHECK (constraint_type IN (
                            'precision', 'nullability', 'type', 'range', 'length', 'choices'
                        )),
    param_max_digits    INTEGER,
    param_decimal_places INTEGER,
    param_max_length    INTEGER,
    param_nullable      INTEGER,
    param_type_name     TEXT,
    param_min_value     REAL,
    param_max_value     REAL,
    param_choices       TEXT,
    source_file         TEXT NOT NULL,
    source_line         INTEGER NOT NULL,
    is_implicit         INTEGER NOT NULL DEFAULT 0,
    verification_level  TEXT NOT NULL DEFAULT 'EXTRACTED'
                        CHECK (verification_level IN ('PROVED', 'TESTED', 'EXTRACTED', 'ASSUMED')),
    contract_role       TEXT CHECK (contract_role IN ('precondition', 'postcondition', NULL)),
    dependent_expr      TEXT,
    subject             TEXT,
    edge_id             INTEGER REFERENCES edges(id)
);

CREATE INDEX idx_contracts_node ON contracts(node_id);
CREATE INDEX idx_edges_source ON edges(source_node_id);
CREATE INDEX idx_edges_target ON edges(target_node_id);
"""

# v1 schema from src/db.rs before data-flow v2 (no qualified_name, subject,
# edge_id, target_param, source_override).
SCHEMA_V1 = """
CREATE TABLE nodes (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    kind        TEXT NOT NULL CHECK (kind IN ('model', 'function', 'field')),
    source_file TEXT NOT NULL,
    source_line INTEGER NOT NULL
);

CREATE TABLE contracts (
    id                  INTEGER PRIMARY KEY,
    node_id             INTEGER NOT NULL REFERENCES nodes(id),
    constraint_type     TEXT NOT NULL CHECK (constraint_type IN (
                            'precision', 'nullability', 'type', 'range', 'length', 'choices'
                        )),
    param_max_digits    INTEGER,
    param_decimal_places INTEGER,
    param_max_length    INTEGER,
    param_nullable      INTEGER,
    param_type_name     TEXT,
    param_min_value     REAL,
    param_max_value     REAL,
    param_choices       TEXT,
    source_file         TEXT NOT NULL,
    source_line         INTEGER NOT NULL,
    is_implicit         INTEGER NOT NULL DEFAULT 0,
    verification_level  TEXT NOT NULL DEFAULT 'EXTRACTED'
                        CHECK (verification_level IN ('PROVED', 'TESTED', 'EXTRACTED', 'ASSUMED')),
    contract_role       TEXT CHECK (contract_role IN ('precondition', 'postcondition', NULL)),
    dependent_expr      TEXT
);

CREATE TABLE edges (
    id              INTEGER PRIMARY KEY,
    source_node_id  INTEGER NOT NULL REFERENCES nodes(id),
    target_node_id  INTEGER NOT NULL REFERENCES nodes(id),
    relationship    TEXT NOT NULL CHECK (relationship IN (
                        'calls', 'writes_to', 'flows_to'
                    )),
    discovery       TEXT NOT NULL CHECK (discovery IN ('ast_pattern', 'manual', 'type_inference'))
);
"""


def node(db, nid, name, kind, qualified=None, v2=True):
    if v2:
        db.execute(
            "INSERT INTO nodes (id, name, kind, source_file, source_line, qualified_name) "
            "VALUES (?, ?, ?, 'app.py', 1, ?)",
            (nid, name, kind, qualified or f"app.{name}"),
        )
    else:
        db.execute(
            "INSERT INTO nodes (id, name, kind, source_file, source_line) "
            "VALUES (?, ?, ?, 'app.py', 1)",
            (nid, name, kind),
        )


def edge(db, eid, src, tgt, rel, target_param=None, override=0):
    db.execute(
        "INSERT INTO edges (id, source_node_id, target_node_id, relationship, discovery, "
        "target_param, source_override) VALUES (?, ?, ?, ?, 'ast_pattern', ?, ?)",
        (eid, src, tgt, rel, target_param, override),
    )


def contract(db, node_id, ctype, role, line, **params):
    cols = ["node_id", "constraint_type", "contract_role", "source_file", "source_line"]
    vals = [node_id, ctype, role, "app.py", line]
    for k, v in params.items():
        cols.append(k)
        vals.append(v)
    db.execute(
        f"INSERT INTO contracts ({', '.join(cols)}) VALUES ({', '.join('?' * len(vals))})",
        vals,
    )


def build_v2(path):
    db = sqlite3.connect(path)
    db.executescript(SCHEMA_V2)
    node(db, 1, "make", "function")
    node(db, 2, "Invoice.total", "model")
    node(db, 3, "Invoice.fee", "model")
    node(db, 4, "with_tax", "function")
    node(db, 5, "rate_of", "function")
    node(db, 6, "combine", "function")
    node(db, 7, "adjust", "function")
    node(db, 8, "Stock.qty", "model")
    node(db, 9, "caller", "function")

    # Field contracts.
    contract(db, 2, "precision", "precondition", 10, param_max_digits=10, param_decimal_places=2)
    contract(db, 3, "precision", "precondition", 11, param_max_digits=10, param_decimal_places=2)
    contract(db, 8, "range", "precondition", 12, param_min_value=0)

    # make: its return value is 6dp, but it writes 2dp to total and 4dp to fee.
    contract(db, 1, "precision", "postcondition", 20, param_decimal_places=6)
    edge(db, 1, 1, 2, "writes_to", override=1)
    edge(db, 2, 1, 3, "writes_to", override=1)
    contract(db, 1, "precision", "postcondition", 21, param_decimal_places=2, edge_id=1)
    contract(db, 1, "precision", "postcondition", 22, param_decimal_places=4, edge_id=2)

    # combine(amount, rate): amount <= 2dp, rate <= 6dp; returns 2dp, written to fee.
    contract(db, 6, "precision", "precondition", 30, param_decimal_places=2, subject="amount")
    contract(db, 6, "precision", "precondition", 31, param_decimal_places=6, subject="rate")
    contract(db, 6, "precision", "postcondition", 32, param_decimal_places=2)
    contract(db, 4, "precision", "postcondition", 33, param_decimal_places=4)
    contract(db, 5, "precision", "postcondition", 34, param_decimal_places=4)
    edge(db, 3, 5, 6, "flows_to", target_param="rate")
    edge(db, 4, 4, 6, "flows_to", target_param="amount")
    edge(db, 5, 6, 3, "writes_to")

    # adjust: range >= -50 and <= 100 written to a PositiveIntegerField.
    contract(db, 7, "range", "postcondition", 40, param_min_value=-50, param_max_value=100)
    edge(db, 6, 7, 8, "writes_to")

    # caller calls make: structural only, never a path.
    edge(db, 7, 9, 1, "calls")
    db.commit()
    db.close()


def build_v1(path):
    db = sqlite3.connect(path)
    db.executescript(SCHEMA_V1)
    node(db, 1, "split_energy", "function", v2=False)
    node(db, 2, "EnergyRecord.energy", "model", v2=False)
    db.execute(
        "INSERT INTO contracts (node_id, constraint_type, param_decimal_places, source_file, "
        "source_line, contract_role) VALUES (1, 'precision', 6, 'utils.py', 42, 'postcondition')"
    )
    db.execute(
        "INSERT INTO contracts (node_id, constraint_type, param_max_digits, param_decimal_places, "
        "source_file, source_line, contract_role) "
        "VALUES (2, 'precision', 5, 3, 'models.py', 15, 'precondition')"
    )
    db.execute(
        "INSERT INTO edges (source_node_id, target_node_id, relationship, discovery) "
        "VALUES (1, 2, 'writes_to', 'ast_pattern')"
    )
    db.commit()
    db.close()


# Round-3 columns: edges.site_file/site_line, contracts.param_min_micros/
# param_max_micros, JSON param_choices; nodes.source_file/source_line are read.
SCHEMA_V3 = (
    SCHEMA_V2.replace(
        "    source_override INTEGER NOT NULL DEFAULT 0\n);",
        "    source_override INTEGER NOT NULL DEFAULT 0,\n"
        "    site_file       TEXT,\n"
        "    site_line       INTEGER\n);",
    ).replace(
        "    edge_id             INTEGER REFERENCES edges(id)\n);",
        "    edge_id             INTEGER REFERENCES edges(id),\n"
        "    param_min_micros    INTEGER,\n"
        "    param_max_micros    INTEGER\n);",
    )
)
assert "site_file" in SCHEMA_V3 and "param_min_micros" in SCHEMA_V3


def node_at(db, nid, name, kind, file, line):
    db.execute(
        "INSERT INTO nodes (id, name, kind, source_file, source_line, qualified_name) "
        "VALUES (?, ?, ?, ?, ?, ?)",
        (nid, name, kind, file, line, f"app.{name}"),
    )


def edge_at(db, eid, src, tgt, rel, site_file=None, site_line=None):
    db.execute(
        "INSERT INTO edges (id, source_node_id, target_node_id, relationship, discovery, "
        "target_param, source_override, site_file, site_line) "
        "VALUES (?, ?, ?, ?, 'ast_pattern', NULL, 0, ?, ?)",
        (eid, src, tgt, rel, site_file, site_line),
    )


def build_v3(path):
    db = sqlite3.connect(path)
    db.executescript(SCHEMA_V3)
    # five() (m.py:9) -> keep(p) (m.py:14, max(input_precision, 2)) -> S.e (3dp),
    # written at w.py:30.
    node_at(db, 1, "five", "function", "m.py", 9)
    node_at(db, 2, "keep", "function", "m.py", 14)
    node_at(db, 3, "S.e", "model", "models.py", 3)
    contract(db, 1, "precision", "postcondition", 10, param_decimal_places=5)
    contract(db, 2, "precision", "postcondition", 15,
             dependent_expr="max(input_precision, 2)")
    contract(db, 3, "precision", "precondition", 4, param_max_digits=10, param_decimal_places=3)
    edge_at(db, 1, 1, 2, "flows_to", "w.py", 29)
    edge_at(db, 2, 2, 3, "writes_to", "w.py", 30)

    # ratio: Field(ge=0.0, le=0.5) (micros); write of 0.7 (micros).
    node_at(db, 4, "make_ratio", "function", "r.py", 1)
    node_at(db, 5, "R.ratio", "model", "models.py", 10)
    contract(db, 4, "range", "postcondition", 2, param_min_value=0.7, param_max_value=0.7,
             param_min_micros=700000, param_max_micros=700000)
    contract(db, 5, "range", "precondition", 11, param_min_value=0.0, param_max_value=0.5,
             param_min_micros=0, param_max_micros=500000)
    edge_at(db, 3, 4, 5, "writes_to", "r.py", 3)

    # legacy REAL only (micros NULL): 0.25 written into le=0.2.
    node_at(db, 6, "make_legacy", "function", "r.py", 20)
    node_at(db, 7, "R.legacy", "model", "models.py", 20)
    contract(db, 6, "range", "postcondition", 21, param_max_value=0.25)
    contract(db, 7, "range", "precondition", 22, param_max_value=0.2)
    edge_at(db, 4, 6, 7, "writes_to", "r.py", 23)

    # choices: CharField(choices=[("a", ...), ("x", ...)]) as a JSON array;
    # one write of "zz", one write without a choices fact.
    node_at(db, 8, "set_zz", "function", "c.py", 1)
    node_at(db, 9, "set_unknown", "function", "c.py", 10)
    node_at(db, 10, "P.status", "model", "models.py", 30)
    contract(db, 8, "choices", "postcondition", 2, param_choices='["zz"]')
    contract(db, 10, "choices", "precondition", 31, param_choices='["a", "x"]')
    edge_at(db, 5, 8, 10, "writes_to", "c.py", 3)
    edge_at(db, 6, 9, 10, "writes_to", "c.py", 11)

    # third() (4dp) written into a 2dp field at three sites: three findings.
    node_at(db, 11, "third", "function", "t.py", 1)
    node_at(db, 12, "P.amount", "model", "models.py", 40)
    contract(db, 11, "precision", "postcondition", 2, param_decimal_places=4)
    contract(db, 12, "precision", "precondition", 41, param_max_digits=10, param_decimal_places=2)
    edge_at(db, 7, 11, 12, "writes_to", "a.py", 10)
    edge_at(db, 8, 11, 12, "writes_to", "b.py", 20)
    edge_at(db, 9, 11, 12, "writes_to", "c.py", 30)
    db.commit()
    db.close()


# Round-5 columns: nodes.is_call_site, contracts.param_min_decimal/param_max_decimal.
SCHEMA_V5 = (
    SCHEMA_V3.replace(
        "    qualified_name TEXT\n);",
        "    qualified_name TEXT,\n"
        "    is_call_site   INTEGER NOT NULL DEFAULT 0\n);",
    ).replace(
        "    param_max_micros    INTEGER\n);",
        "    param_max_micros    INTEGER,\n"
        "    param_min_decimal   TEXT,\n"
        "    param_max_decimal   TEXT\n);",
    )
)
assert "is_call_site" in SCHEMA_V5 and "param_max_decimal" in SCHEMA_V5


def build_v5(path):
    db = sqlite3.connect(path)
    db.executescript(SCHEMA_V5)
    # big(): le=2e13 written into le=1e13 (exact decimals, no micros).
    node_at(db, 1, "big", "function", "d.py", 1)
    node_at(db, 2, "D.big", "model", "models.py", 1)
    contract(db, 1, "range", "postcondition", 2, param_max_decimal="20000000000000")
    contract(db, 2, "range", "precondition", 3, param_max_decimal="10000000000000")
    edge_at(db, 1, 1, 2, "writes_to", "d.py", 4)
    # seven(): 0.1234567 into le=0.1234567 (consistent); eight(): 0.1234568 (error).
    node_at(db, 3, "seven", "function", "d.py", 10)
    node_at(db, 4, "eight", "function", "d.py", 20)
    node_at(db, 5, "D.fine", "model", "models.py", 10)
    contract(db, 3, "range", "postcondition", 11, param_max_decimal="0.1234567",
             param_max_micros=123457, param_max_value=0.1234567)
    contract(db, 4, "range", "postcondition", 21, param_max_decimal="0.1234568")
    contract(db, 5, "range", "precondition", 12, param_max_decimal="0.1234567",
             param_max_micros=123456)
    edge_at(db, 2, 3, 5, "writes_to", "d.py", 13)
    edge_at(db, 3, 4, 5, "writes_to", "d.py", 23)
    # micros only (0.5000001 can't be given in micros: 0.500001) into le=0.5000009.
    node_at(db, 6, "micro", "function", "d.py", 30)
    contract(db, 6, "range", "postcondition", 31, param_max_micros=500001)
    node_at(db, 7, "D.mix", "model", "models.py", 30)
    contract(db, 7, "range", "precondition", 32, param_max_decimal="0.5000009")
    edge_at(db, 4, 6, 7, "writes_to", "d.py", 33)
    # caller (3dp) -> call site cs (max(input_precision, 2)) -> D.p (2dp).
    node_at(db, 8, "caller", "function", "c.py", 1)
    node_at(db, 9, "cs", "function", "c.py", 5)
    db.execute("UPDATE nodes SET is_call_site = 1 WHERE id = 9")
    node_at(db, 10, "D.p", "model", "models.py", 40)
    contract(db, 8, "precision", "postcondition", 2, param_decimal_places=3)
    contract(db, 9, "precision", "postcondition", 6, dependent_expr="max(input_precision, 2)")
    contract(db, 10, "precision", "precondition", 41, param_max_digits=10, param_decimal_places=2)
    edge_at(db, 5, 8, 9, "flows_to", "c.py", 3)
    edge_at(db, 6, 9, 10, "writes_to", "c.py", 7)
    db.commit()
    db.close()


def check_v5(checker, tmp):
    v5 = os.path.join(tmp, "v5.sqlite")
    build_v5(v5)
    code, out = run(checker, v5)
    results = out["results"]
    for r in results:
        print("     ", r["severity"], " -> ".join(r["path"]), r["source_guarantee"], "|",
              r["target_requirement"], r["guarantee_at"])
    errs = sorted((" -> ".join(r["path"]), r["source_guarantee"], r["target_requirement"])
                  for r in results if r["severity"] == "error")
    check(errs == sorted([
        ("big -> D.big", "range ≤ 20000000000000", "range ≤ 10000000000000"),
        ("eight -> D.fine", "range ≤ 0.1234568", "range ≤ 0.1234567"),
        ("micro -> D.mix", "range ≤ 0.500001", "range ≤ 0.5000009"),
        ("caller -> cs -> D.p", "precision ≤ 3", "precision ≤ 2"),
    ]), "v5: exact decimal errors (2e13 > 1e13, 7 places, mixed with micros) and the call-site path")
    check(code == 1, "v5: exit code 1")
    check([r for r in results if r["severity"] == "warning"] == [],
          "v5: no unresolved-bound warning on the call-site suffix cs -> D.p")
    big = [r for r in results if r["path"] == ["big", "D.big"]][0]
    check(big["guarantee_at"] == {"file": "app.py", "line": 2}, "v5: guarantee_at is the guarantee's row")
    check(out["summary"]["states_checked"] == out["summary"]["paths_checked"] > 0,
          "v5: states_checked reported")


def check_v3(checker, tmp):
    v3 = os.path.join(tmp, "v3.sqlite")
    build_v3(v3)
    code, out = run(checker, v3)
    results = out["results"]
    errors = [r for r in results if r["severity"] == "error"]
    warnings = [r for r in results if r["severity"] == "warning"]
    for r in results:
        print("     ", r["severity"], " -> ".join(r["path"]), r["source_guarantee"], "|",
              r["target_requirement"], r["source"], r["site"])
    check(code == 1 and out["exit_code"] == 1, "v3: exit code 1")
    check(all(set(r["site"]) == {"file", "line"} for r in results), "v3: every result has a site")

    comp = [r for r in errors if r["path"] == ["five", "keep", "S.e"]]
    check(len(comp) == 1 and comp[0]["source"] == {"file": "m.py", "line": 9, "name": "five"},
          "v3: composed error located at the path head's definition (M1)")
    check(comp[0]["site"] == {"file": "w.py", "line": 30} and comp[0]["hop"] == ["keep", "S.e"],
          "v3: composed error carries the failing hop's site")
    unres = [r for r in warnings if r["hop"] == ["keep", "S.e"]]
    check(len(unres) == 1 and unres[0]["source"] == {"file": "m.py", "line": 14, "name": "keep"}
          and unres[0]["target_requirement"] == "precision ≤ 3",
          "v3: unresolved warning at the hop source, showing the requirement (M2, M4)")

    ratio = [(r["source_guarantee"], r["target_requirement"]) for r in errors
             if r["path"] == ["make_ratio", "R.ratio"]]
    check(ratio == [("range ≤ 0.7", "range ≤ 0.5")], "v3: micros bounds compare 0.7 > 0.5")
    legacy = [(r["source_guarantee"], r["target_requirement"]) for r in errors
              if r["path"] == ["make_legacy", "R.legacy"]]
    check(legacy == [("range ≤ 0.25", "range ≤ 0.2")], "v3: legacy REAL bounds fall back")

    zz = [(r["source_guarantee"], r["target_requirement"]) for r in errors
          if r["path"] == ["set_zz", "P.status"]]
    check(zz == [("choices in [zz]", "choices in [a, x]")], "v3: JSON choices subset check")
    unknown = [r for r in warnings if r["path"] == ["set_unknown", "P.status"]]
    check(len(unknown) == 1 and unknown[0]["source_guarantee"] == "choices (unspecified)",
          "v3: write without a choices fact warns")

    sites = sorted((r["site"]["file"], r["site"]["line"]) for r in errors
                   if r["path"] == ["third", "P.amount"])
    check(sites == [("a.py", 10), ("b.py", 20), ("c.py", 30)],
          "v3: findings at different sites are not merged (M3)")

    # State budget (--max-states; --max-paths is its alias): 10 hop states
    # (every edge once, and keep -> S.e also composed after five); a budget
    # of 3 is exceeded.
    proc = subprocess.run([checker, v3], capture_output=True, text=True)
    states = json.loads(proc.stdout)["summary"]["paths_checked"]
    check(states == 10, "v3: 10 hop states checked")
    for flag in ["--max-paths", "--max-states"]:
        proc = subprocess.run([checker, v3, flag, "3"], capture_output=True, text=True)
        out = json.loads(proc.stdout)
        check(proc.returncode == 2 and out["exit_code"] == 2, f"v3: {flag} exceeded exits 2")
        check([(r["status"], r["severity"]) for r in out["results"]] == [("incomplete", "error")]
              and "--max-states 3" in out["results"][0]["suggestion"],
              f"v3: {flag}: one incomplete error naming the budget")
        check(out["summary"]["paths_checked"] == 0 and out["summary"]["edges_checked"] == 9,
              f"v3: {flag}: incomplete summary")
    proc = subprocess.run([checker, v3, "--max-paths", str(states)], capture_output=True, text=True)
    check(proc.returncode == 1 and json.loads(proc.stdout)["summary"]["paths_checked"] == states,
          "v3: a budget equal to the state count is not exceeded")
    proc = subprocess.run([checker, v3, "--max-states-per-edge", "2", "--max-states", "100"],
                          capture_output=True, text=True)
    check(proc.returncode == 1, "v3: at most two states per edge fit a per-edge cap of 2")
    proc = subprocess.run([checker, v3, "--max-states-per-edge", "1"], capture_output=True, text=True)
    out = json.loads(proc.stdout)
    check(proc.returncode == 2 and "keep -> S.e" in out["results"][0]["suggestion"],
          "v3: per-edge cap 1 exceeded on keep -> S.e (raw and composed)")
    proc = subprocess.run([checker, v3, "--max-paths", "many"], capture_output=True, text=True)
    check(proc.returncode == 2 and proc.stdout == "", "v3: bad --max-paths is a usage error")
    proc = subprocess.run([checker, v3, "--max-states-per-edge"], capture_output=True, text=True)
    check(proc.returncode == 2 and proc.stdout == "", "v3: missing option value is a usage error")


def run(checker, db_path):
    proc = subprocess.run([checker, db_path], capture_output=True, text=True)
    if proc.stderr:
        print(proc.stderr, file=sys.stderr)
    return proc.returncode, json.loads(proc.stdout)


def check(cond, msg):
    if not cond:
        print(f"FAIL: {msg}")
        sys.exit(1)
    print(f"ok    {msg}")


def main():
    checker = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_CHECKER
    if not os.access(checker, os.X_OK):
        print(f"missing checker {checker} (cd prover && lake build)", file=sys.stderr)
        sys.exit(2)
    with tempfile.TemporaryDirectory() as tmp:
        v2 = os.path.join(tmp, "v2.sqlite")
        build_v2(v2)
        code, out = run(checker, v2)
        results = out["results"]
        errors = [r for r in results if r["severity"] == "error"]
        warnings = [r for r in results if r["severity"] == "warning"]
        found = sorted(
            (" -> ".join(r["path"]), r["source_guarantee"], r["target_requirement"],
             r["target"]["name"], tuple(r["hop"]))
            for r in errors
        )
        expected = sorted([
            ("make -> Invoice.fee", "precision ≤ 4", "precision ≤ 2",
             "Invoice.fee", ("make", "Invoice.fee")),
            ("with_tax -> combine", "precision ≤ 4", "precision ≤ 2",
             "combine", ("with_tax", "combine")),
            ("adjust -> Stock.qty", "range ≥ -50", "range ≥ 0",
             "Stock.qty", ("adjust", "Stock.qty")),
        ])
        for f in found:
            print("      error:", f)
        check(found == expected, "v2: errors are the override, target_param and lower-bound findings")
        check(code == 1 and out["exit_code"] == 1, "v2: exit code 1")
        check([r["witness"] for r in errors if r["hop"] == ["with_tax", "combine"]]
              == [["with_tax", "combine", "Invoice.fee"]],
              "v2: path ends at the failing hop; the witness goes on to the model")
        check(all(r["path"][0] != "caller" for r in results), "v2: calls edge is not followed")
        check(not any("make -> Invoice.total" == " -> ".join(r["path"]) for r in errors),
              "v2: make's 2dp write to Invoice.total is consistent (override, not its 6dp return)")
        check(not any(r["hop"][0] == "rate_of" for r in errors),
              "v2: rate_of's 4dp value is checked only against combine's rate requirement")
        check(warnings == [], "v2: no warnings")
        check(all("hop" in r and len(r["hop"]) == 2 for r in results), "v2: every result has a hop")

        v1 = os.path.join(tmp, "v1.sqlite")
        build_v1(v1)
        code, out = run(checker, v1)
        errs = [(" -> ".join(r["path"]), r["source_guarantee"], r["target_requirement"])
                for r in out["results"] if r["severity"] == "error"]
        check(errs == [("split_energy -> EnergyRecord.energy", "precision ≤ 6", "precision ≤ 3")]
              and code == 1, "v1 schema still translates")
        check(all(r["site"] == {"file": "", "line": 0} for r in out["results"]),
              "v1: no site columns, empty sites")
        check(errs and out["results"][0]["source"]["line"] == 1,
              "v1: error source is the head node's definition")

        check_v3(checker, tmp)
        check_v5(checker, tmp)
    print("translation smoke test passed")


if __name__ == "__main__":
    main()
