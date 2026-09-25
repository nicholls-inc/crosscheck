#!/usr/bin/env python3
"""Translation smoke test for the data-flow v2 SQLite interface.

Builds a SQLite database with the v2 schema (docs/design/dataflow-v2.md on top
of the schema in src/db.rs), runs the Lean checker on it and asserts on the
JSON. Covers a per-edge source override, target_param filtering, a range row
with a lower bound, a calls edge (not followed) and hop attribution. Also runs
the checker on a database with the v1 schema (no v2 columns), which must still
translate.

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
            ("with_tax -> combine -> Invoice.fee", "precision ≤ 4", "precision ≤ 2",
             "combine", ("with_tax", "combine")),
            ("adjust -> Stock.qty", "range ≥ -50", "range ≥ 0",
             "Stock.qty", ("adjust", "Stock.qty")),
        ])
        for f in found:
            print("      error:", f)
        check(found == expected, "v2: errors are the override, target_param and lower-bound findings")
        check(code == 1 and out["exit_code"] == 1, "v2: exit code 1")
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
    print("translation smoke test passed")


if __name__ == "__main__":
    main()
