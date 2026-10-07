import { describe, it, expect, vi, afterEach } from "vitest";
import { execFileSync, spawnSync } from "node:child_process";
import { rm } from "node:fs/promises";
import { fileURLToPath } from "node:url";

vi.mock("../../docker.js", async (importOriginal) => ({
  SANDBOX_FLAGS: (await importOriginal<typeof import("../../docker.js")>()).SANDBOX_FLAGS,
  getDockerImage: vi.fn(() => "crosscheck-dafny:latest"),
  dockerImageId: vi.fn(),
  runDafny: vi.fn(),
}));

import { dockerImageId, runDafny } from "../../docker.js";
import { dafnyEvidence, rerunCommand, type EvidenceOutput } from "../../tools/evidence.js";
import { commitProgram, readDafnyOutput, type DafnyOutput } from "../fixtures/dafny-output.js";

const CHECKER = fileURLToPath(new URL("../../../../../scripts/check-evidence-record.mjs", import.meta.url));
const IMAGE_ID = "sha256:ccd363af0c7fdba1a7766568daf1ded858f93708eac6bb7f01b7d1ee725d8f1c";
const VERSION = "4.11.0+fcb2042d6d043a2634f0854338c08feeaaaf4ae2";
const NOT_VERIFIED = (t: string) =>
  `theorem not verified in Abs.dfy or its includes: ${t}; name it as Dafny's verification log does, qualified by every enclosing module and type`;

const repos: string[] = [];

// Replays the runs real Dafny 4.11.0 made on the program, in order, and fails on a run whose arguments
// differ from the recorded ones, so the fixtures stay the output of the calls the tool makes now.
async function replay(
  name: string,
  theorems?: string[],
  outputPath?: string
): Promise<{ result: EvidenceOutput; repo: string; output: DafnyOutput }> {
  const output = await readDafnyOutput(name);
  const repo = await commitProgram(output.program, "evidence-dafny-output-");
  repos.push(repo);
  vi.mocked(dockerImageId).mockResolvedValue(output.imageId);
  const runs = [...output.runs];
  vi.mocked(runDafny).mockImplementation(async (_dir, args) => {
    const next = runs.shift();
    if (next === undefined) throw new Error(`no recorded Dafny run left for ${args.join(" ")}`);
    expect(args).toEqual(next.args);
    return { exitCode: next.exitCode, stdout: next.stdout, stderr: next.stderr, timedOut: next.timedOut };
  });
  const result = await dafnyEvidence({
    repoPath: repo,
    file: output.program.file,
    statement: "A statement.",
    requirement: null,
    theorems: theorems ?? output.program.theorems,
    outputPath,
  });
  return { result, repo, output };
}

function head(repo: string): string {
  return execFileSync("git", ["-C", repo, "rev-parse", "HEAD"]).toString().trim();
}

describe("dafnyEvidence on recorded output of real Dafny 4.11.0", () => {
  afterEach(async () => {
    for (const r of repos.splice(0)) await rm(r, { recursive: true, force: true });
  });

  it("emits a record that the evidence record checker accepts (DE-8, DE-10)", async () => {
    const { result, repo } = await replay("proved", undefined, "record.json");
    expect(result.errors).toEqual([]);
    expect(result.record).toEqual({
      format: "evidence-record/1",
      commit: head(repo),
      claims: [
        {
          id: "dafny-abs",
          statement: "A statement.",
          requirement: null,
          strength: "proved",
          basis: { theorems: ["AbsNonneg"] },
          trusted_base: [
            { component: "Dafny verifier", version: VERSION },
            { component: "Z3 solver shipped with the Dafny release", version: `Dafny ${VERSION}` },
            { component: "Dafny Docker image crosscheck-dafny:latest", version: IMAGE_ID },
          ],
          rerun: { command: rerunCommand(IMAGE_ID, ["Abs.dfy"]), exit_code: 0 },
        },
      ],
    });
    const check = spawnSync(process.execPath, [CHECKER, result.writtenTo!], { encoding: "utf-8" });
    expect({ status: check.status, stdout: check.stdout, stderr: check.stderr }).toEqual({
      status: 0,
      stdout: "",
      stderr: "",
    });
  });

  it("accepts a theorem in a class in a module only by its qualified name (DE-5)", async () => {
    expect((await replay("qualified-name", ["M.C.L"])).result.errors).toEqual([]);
    expect((await replay("qualified-name", ["L", "M.L", "M.C.L"])).result.errors).toEqual([
      NOT_VERIFIED("L"),
      NOT_VERIFIED("M.L"),
    ]);
  });

  it("accepts a theorem declared in an include, and audits the include (DE-5, DE-7)", async () => {
    const { result } = await replay("include-proved");
    expect(result.errors).toEqual([]);
    expect(result.record!.claims[0].basis.theorems).toEqual(["T", "Ok"]);
    expect(result.record!.claims[0].rerun.command).toBe(rerunCommand(IMAGE_ID, ["Abs.dfy", "Lib.dfy"]));
  });

  it("refuses a theorem that is not in the log of a passing run (DE-5)", async () => {
    expect((await replay("include-proved", ["T", "Missing"])).result.errors).toEqual([NOT_VERIFIED("Missing")]);
  });

  it("refuses a theorem that rests on an unproved included lemma, though the log passes it (DE-6)", async () => {
    const { result, output } = await replay("include-unproved");
    expect(output.runs[0].stdout).toContain("T (correctness),Passed,");
    expect(result.success).toBe(false);
    expect(result.errors[0]).toBe("dafny verify exited 4");
    expect(result.errors[1]).toContain("Lib.dfy(1,26): Error: a postcondition could not be proved");
    expect(result.record).toBeNull();
  });

  it("refuses a failing postcondition (DE-6)", async () => {
    const { result } = await replay("failing-postcondition");
    expect(result.errors[0]).toBe("dafny verify exited 4");
    expect(result.errors[1]).toContain("Neg (correctness),Failed,");
  });

  it("refuses an assume, which fails the run on its warning though the log passes it (DE-6)", async () => {
    const { result, output } = await replay("assume");
    expect(output.runs[0].stdout).toContain("Pos (correctness),Passed,");
    expect(result.errors[0]).toBe("dafny verify exited 2");
    expect(result.errors[1]).toContain("Warning: assume statement has no {:axiom} annotation");
  });

  it("refuses an {:axiom} lemma that verify accepts with exit 0 (DE-7)", async () => {
    const { result, output } = await replay("axiom");
    expect(output.runs[0].exitCode).toBe(0);
    expect(result.errors).toEqual([
      "dafny audit did not report 0 findings",
      "Abs.dfy(1,15): Warning: Bad: Declaration has explicit `{:axiom}` attribute. Possible mitigation: Provide a proof or test.\n" +
        "Dafny auditor completed with 1 findings\n\nDafny program verifier did not attempt verification",
    ]);
  });

  it("refuses a theorem that rests on an included {:axiom}, though verify and its log pass it (DE-7)", async () => {
    const { result, output } = await replay("include-axiom");
    expect(output.runs[0]).toMatchObject({ exitCode: 0, stdout: expect.stringContaining("T (correctness),Passed,") });
    expect(result.errors[0]).toBe("dafny audit did not report 0 findings");
    expect(result.errors[1]).toContain("Lib.dfy(1,15): Warning: Bad: Declaration has explicit `{:axiom}` attribute.");
  });
});
