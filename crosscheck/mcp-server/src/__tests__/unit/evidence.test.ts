import { describe, it, expect } from "vitest";
import { execFileSync, spawnSync } from "node:child_process";
import { chmodSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import {
  buildRecord,
  claimId,
  rerunCommand,
  shellQuote,
  unverifiedTheorems,
  validateEvidenceInput,
  type EvidenceInput,
} from "../../tools/evidence.js";

const good: EvidenceInput = {
  repoPath: "/repo",
  file: "proofs/Abs.dfy",
  statement: "Abs never returns a negative number.",
  requirement: null,
  theorems: ["AbsNonneg"],
};

describe("validateEvidenceInput (DE-1)", () => {
  it("accepts a well-formed input", () => {
    expect(validateEvidenceInput(good)).toEqual([]);
  });

  it.each([
    ["relative repoPath", { repoPath: "repo" }],
    ["absolute file", { file: "/proofs/Abs.dfy" }],
    ["parent segment", { file: "../Abs.dfy" }],
    ["dot segment", { file: "./Abs.dfy" }],
    ["empty segment", { file: "proofs//Abs.dfy" }],
    ["backslash", { file: "proofs\\Abs.dfy" }],
    ["wrong extension", { file: "proofs/Abs.lean" }],
    ["blank statement", { statement: " \n" }],
    ["blank requirement", { requirement: "  " }],
    ["no theorems", { theorems: [] }],
    ["bad theorem name", { theorems: ["Abs Nonneg"] }],
    ["trailing dot", { theorems: ["M."] }],
  ])("refuses %s", (_label, patch) => {
    expect(validateEvidenceInput({ ...good, ...patch })).toHaveLength(1);
  });

  it("reports every problem", () => {
    expect(
      validateEvidenceInput({ ...good, repoPath: "x", statement: "", theorems: [] })
    ).toHaveLength(3);
  });

  it("accepts primes, question marks and qualifiers in names", () => {
    expect(validateEvidenceInput({ ...good, theorems: ["M.N.f'", "Valid?"] })).toEqual([]);
  });
});

describe("unverifiedTheorems (DE-5)", () => {
  const log = [
    "Dafny program verifier finished with 4 verified, 0 errors",
    "TestResult.DisplayName,TestResult.Outcome,TestResult.Duration,TestResult.ResourceCount,RandomSeed",
    "Top (correctness),Passed,00:00:00.1097333,2473,0",
    "M.C.Mm (correctness),Passed,00:00:00.0744907,4122,0",
    "M.L (correctness),Passed,00:00:00.0263585,3461,0",
    "M.F' (well-formedness),Passed,00:00:00.0137694,3053,0",
    "M.Bad (correctness),Failed,00:00:00.0137694,3053,0",
    "Results File: /dev/stdout",
  ].join("\n");

  it("accepts the fully qualified names the log reports as passed", () => {
    expect(unverifiedTheorems(log, ["Top", "M.C.Mm", "M.L", "M.F'"])).toEqual([]);
  });

  it("refuses an unqualified, a partly qualified, an over-qualified and a failed name", () => {
    expect(unverifiedTheorems(log, ["L", "C.Mm", "M.Top", "_module.Top", "M.Bad", "M"])).toEqual([
      "L",
      "C.Mm",
      "M.Top",
      "_module.Top",
      "M.Bad",
      "M",
    ]);
  });

  it("reads names only after the log header", () => {
    expect(unverifiedTheorems("Spoof (correctness),Passed,0,0,0\n", ["Spoof"])).toEqual(["Spoof"]);
    expect(
      unverifiedTheorems("Spoof (correctness),Passed,0,0,0\nTestResult.DisplayName,x\nReal (correctness),Passed,0,0,0", [
        "Spoof",
        "Real",
      ])
    ).toEqual(["Spoof"]);
  });
});

describe("rerunCommand (DE-9)", () => {
  it("is the exact command for a plain path", () => {
    expect(rerunCommand("crosscheck-dafny:latest", ["proofs/Abs.dfy"])).toBe(
      `docker run --rm --network=none -v "$PWD":/work:ro 'crosscheck-dafny:latest' verify '/work/proofs/Abs.dfy' --verify-included-files && ` +
        `out=$(docker run --rm --network=none -v "$PWD":/work:ro 'crosscheck-dafny:latest' audit '/work/proofs/Abs.dfy' 2>&1) && ` +
        `case "$out" in *'Dafny auditor completed with 0 findings'*) true ;; *) false ;; esac`
    );
  });

  it("verifies the first file with its includes and audits every file", () => {
    expect(rerunCommand("img", ["proofs/Abs.dfy", "proofs/Lib.dfy"])).toBe(
      `docker run --rm --network=none -v "$PWD":/work:ro 'img' verify '/work/proofs/Abs.dfy' --verify-included-files && ` +
        `out=$(docker run --rm --network=none -v "$PWD":/work:ro 'img' audit '/work/proofs/Abs.dfy' '/work/proofs/Lib.dfy' 2>&1) && ` +
        `case "$out" in *'Dafny auditor completed with 0 findings'*) true ;; *) false ;; esac`
    );
  });

  it.each([
    ["verify and audit pass", 0, 0, "Dafny auditor completed with 0 findings", 0],
    ["verify fails", 4, 0, "Dafny auditor completed with 0 findings", 4],
    ["the audit exits non-zero with the clean line", 0, 3, "Dafny auditor completed with 0 findings", 3],
    ["the audit reports a finding", 0, 0, "Dafny auditor completed with 1 findings", 1],
  ])("exits as Dafny does when %s", (_label, verifyExit, auditExit, auditLine, expected) => {
    const bin = mkdtempSync(join(tmpdir(), "fake-docker-"));
    try {
      writeFileSync(
        join(bin, "docker"),
        `#!/bin/sh\nfor a; do case "$a" in verify|audit) mode=$a ;; esac; done\n` +
          `case "$mode" in verify) exit ${verifyExit} ;; audit) echo '${auditLine}' >&2; exit ${auditExit} ;; esac\nexit 99\n`
      );
      chmodSync(join(bin, "docker"), 0o755);
      const run = spawnSync("sh", ["-c", rerunCommand("img", ["Abs.dfy"])], {
        env: { ...process.env, PATH: `${bin}:${process.env.PATH}` },
      });
      expect(run.status).toBe(expected);
    } finally {
      rmSync(bin, { recursive: true, force: true });
    }
  });

  it("quotes a quote and a space so the shell passes the path through", () => {
    const path = "it's a/proof.dfy";
    const out = execFileSync("sh", ["-c", `printf %s ${shellQuote(path)}`]).toString();
    expect(out).toBe(path);
    expect(rerunCommand("img", [path])).toContain(`'/work/it'\\''s a/proof.dfy'`);
  });
});

describe("claimId (DE-10)", () => {
  it.each([
    ["proofs/Abs.dfy", "dafny-proofs-abs"],
    ["A__B..c.dfy", "dafny-a-b-c"],
    ["_x_.dfy", "dafny-x"],
    ["__.dfy", "dafny"],
  ])("%s -> %s", (file, id) => {
    expect(claimId(file)).toBe(id);
  });
});

describe("buildRecord (DE-8, DE-10)", () => {
  it("builds the literal record", () => {
    const record = buildRecord({
      commit: "0123456789abcdef0123456789abcdef01234567",
      file: "proofs/Abs.dfy",
      includes: ["proofs/Lib.dfy"],
      statement: "  Abs never returns a negative number. ",
      requirement: " docs/req.md#abs ",
      theorems: ["Arith.Abs", "AbsNonneg"],
      dafnyVersion: "4.11.0+fcb2042d",
      image: "crosscheck-dafny:latest",
      imageId: "sha256:feed",
    });
    expect(JSON.parse(JSON.stringify(record))).toEqual({
      format: "evidence-record/1",
      commit: "0123456789abcdef0123456789abcdef01234567",
      claims: [
        {
          id: "dafny-proofs-abs",
          statement: "Abs never returns a negative number.",
          requirement: "docs/req.md#abs",
          strength: "proved",
          basis: { theorems: ["Arith.Abs", "AbsNonneg"] },
          trusted_base: [
            { component: "Dafny verifier", version: "4.11.0+fcb2042d" },
            { component: "Z3 solver shipped with the Dafny release", version: "Dafny 4.11.0+fcb2042d" },
            { component: "Dafny Docker image crosscheck-dafny:latest", version: "sha256:feed" },
          ],
          rerun: {
            command: rerunCommand("crosscheck-dafny:latest", ["proofs/Abs.dfy", "proofs/Lib.dfy"]),
            exit_code: 0,
          },
        },
      ],
    });
    expect(Object.keys(record.claims[0])).toEqual([
      "id",
      "statement",
      "requirement",
      "strength",
      "basis",
      "trusted_base",
      "rerun",
    ]);
  });
});
