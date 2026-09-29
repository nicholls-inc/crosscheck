#!/usr/bin/env bash
# Print the findings (errors and warnings) present in HEAD.json but not in
# BASE.json, for reviewing a pull request. Both files are the JSON printed by
# `crosscheck-contracts contracts check`. A finding is keyed by
# (path, severity, source_guarantee, target_requirement), so a moved line does
# not make it new. Each new finding is one line with its HEAD site. Exits 0.
#
# Usage: scripts/diff-findings.sh BASE.json HEAD.json
set -euo pipefail

[ "$#" -eq 2 ] || { echo "usage: $0 BASE.json HEAD.json" >&2; exit 2; }

python3 - "$1" "$2" <<'PY'
import json, sys

def findings(path):
    out = {}
    for r in json.load(open(path))["results"]:
        if r["severity"] not in ("error", "warning"):
            continue
        key = (" -> ".join(r["path"]), r["severity"], r["source_guarantee"], r["target_requirement"])
        out.setdefault(key, r)
    return out

base, head = findings(sys.argv[1]), findings(sys.argv[2])
for key in sorted(set(head) - set(base)):
    path, severity, guarantee, requirement = key
    site = head[key]["site"]
    print(f"{severity}: {path} | {guarantee} -> {requirement} | {site['file']}:{site['line']}")
PY
