import { describe, it, expect } from "vitest";
import { execFileSync } from "node:child_process";
import {
  buildRecord,
  claimId,
  rerunCommand,
  shellQuote,
  undeclaredTheorems,
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

describe("undeclaredTheorems (DE-5)", () => {
  const source = `
module Arith {
  function Abs(x: int): int { if x < 0 then -x else x }
  lemma {:induction false} {:isolate_assertions} AbsNonneg(x: int) ensures Abs(x) >= 0 {}
  ghost predicate Pos?(x: int) { x > 0 }
  twostate lemma Frame'() {}
}
method Main() {}
lemma Step_2() {}
`;

  it("finds declarations after keywords and attributes", () => {
    expect(
      undeclaredTheorems(source, ["Abs", "AbsNonneg", "Pos?", "Frame'", "Main", "Arith.AbsNonneg"])
    ).toEqual([]);
  });

  it("rejects a name that only prefixes a declaration", () => {
    expect(undeclaredTheorems(source, ["AbsNon", "Pos", "Frame", "Step"])).toEqual([
      "AbsNon",
      "Pos",
      "Frame",
      "Step",
    ]);
  });

  it("rejects a name used but not declared", () => {
    expect(undeclaredTheorems("lemma L() ensures Helper() {}", ["Helper"])).toEqual(["Helper"]);
  });

  it("rejects an undeclared module qualifier", () => {
    expect(undeclaredTheorems(source, ["Other.AbsNonneg"])).toEqual(["Other.AbsNonneg"]);
  });
});

describe("rerunCommand (DE-9)", () => {
  it("is the exact command for a plain path", () => {
    expect(rerunCommand("crosscheck-dafny:latest", "proofs/Abs.dfy")).toBe(
      `docker run --rm --network=none -v "$PWD":/work 'crosscheck-dafny:latest' verify '/work/proofs/Abs.dfy' && ` +
        `docker run --rm --network=none -v "$PWD":/work 'crosscheck-dafny:latest' audit '/work/proofs/Abs.dfy' 2>&1 | ` +
        `grep -q 'Dafny auditor completed with 0 findings'`
    );
  });

  it("quotes a quote and a space so the shell passes the path through", () => {
    const path = "it's a/proof.dfy";
    const out = execFileSync("sh", ["-c", `printf %s ${shellQuote(path)}`]).toString();
    expect(out).toBe(path);
    expect(rerunCommand("img", path)).toContain(`'/work/it'\\''s a/proof.dfy'`);
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
      statement: "  Abs never returns a negative number. ",
      requirement: " docs/req.md#abs ",
      theorems: ["AbsNonneg", "Arith.Abs"],
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
          basis: { theorems: ["AbsNonneg", "Arith.Abs"] },
          trusted_base: [
            { component: "Dafny verifier", version: "4.11.0+fcb2042d" },
            { component: "Z3 solver shipped with the Dafny release", version: "Dafny 4.11.0+fcb2042d" },
            { component: "Dafny Docker image crosscheck-dafny:latest", version: "sha256:feed" },
          ],
          rerun: {
            command: rerunCommand("crosscheck-dafny:latest", "proofs/Abs.dfy"),
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
