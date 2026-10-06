import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { execFileSync } from "node:child_process";
import { mkdtemp, mkdir, readFile, rm, symlink, writeFile } from "node:fs/promises";
import { realpathSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";

vi.mock("../../docker.js", () => ({
  getDockerImage: vi.fn(() => "crosscheck-dafny:latest"),
  dockerImageId: vi.fn(),
  runDafny: vi.fn(),
}));

import { dockerImageId, runDafny } from "../../docker.js";
import { dafnyEvidence, rerunCommand, type EvidenceInput } from "../../tools/evidence.js";

const AUDIT_CLEAN = "Dafny auditor completed with 0 findings";

type Run = { exitCode: number; stdout: string; stderr: string; timedOut: boolean };
const ok = (stdout: string): Run => ({ exitCode: 0, stdout, stderr: "", timedOut: false });

function stubDafny(over: Partial<Record<"verify" | "audit" | "--version", Run>> = {}) {
  const runs: Record<string, Run> = {
    verify: ok("Dafny program verifier finished with 1 verified, 0 errors\n"),
    audit: ok(`${AUDIT_CLEAN}\n\nDafny program verifier did not attempt verification\n`),
    "--version": ok("4.11.0+fcb2042d\n"),
    ...over,
  };
  vi.mocked(runDafny).mockImplementation(async (_dir, args) => runs[args[0]]);
  vi.mocked(dockerImageId).mockResolvedValue("sha256:feed");
}

function git(cwd: string, ...args: string[]): string {
  return execFileSync("git", ["-C", cwd, ...args]).toString().trim();
}

const SOURCE = "function Abs(x: int): int { if x < 0 then -x else x }\nlemma AbsNonneg(x: int) ensures Abs(x) >= 0 {}\n";

describe("dafnyEvidence against a real git repository", () => {
  let repo: string;
  let commit: string;
  let input: EvidenceInput;

  beforeEach(async () => {
    repo = realpathSync(await mkdtemp(join(tmpdir(), "evidence-")));
    git(repo, "init", "-q");
    git(repo, "config", "user.email", "t@example.com");
    git(repo, "config", "user.name", "t");
    await mkdir(join(repo, "proofs"));
    await writeFile(join(repo, "proofs", "Abs.dfy"), SOURCE);
    await writeFile(join(repo, ".gitignore"), "out/\n");
    git(repo, "add", ".");
    git(repo, "commit", "-q", "-m", "init");
    commit = git(repo, "rev-parse", "HEAD");
    input = {
      repoPath: join(repo, "proofs"),
      file: "proofs/Abs.dfy",
      statement: "Abs never returns a negative number.",
      requirement: null,
      theorems: ["AbsNonneg"],
    };
    stubDafny();
  });

  afterEach(async () => {
    await rm(repo, { recursive: true, force: true });
  });

  it("emits the record from a subdirectory and writes it under the top level", async () => {
    await mkdir(join(repo, "out"));
    const result = await dafnyEvidence({ ...input, outputPath: "out/record.json" });

    const expected = {
      format: "evidence-record/1",
      commit,
      claims: [
        {
          id: "dafny-proofs-abs",
          statement: "Abs never returns a negative number.",
          requirement: null,
          strength: "proved",
          basis: { theorems: ["AbsNonneg"] },
          trusted_base: [
            { component: "Dafny verifier", version: "4.11.0+fcb2042d" },
            { component: "Z3 solver shipped with the Dafny release", version: "Dafny 4.11.0+fcb2042d" },
            { component: "Dafny Docker image crosscheck-dafny:latest", version: "sha256:feed" },
          ],
          rerun: { command: rerunCommand("crosscheck-dafny:latest", "proofs/Abs.dfy"), exit_code: 0 },
        },
      ],
    };
    expect(result).toEqual({
      success: true,
      errors: [],
      record: expected,
      writtenTo: join(repo, "out", "record.json"),
    });
    expect(commit).toMatch(/^[0-9a-f]{40}$/);
    expect(await readFile(join(repo, "out", "record.json"), "utf-8")).toBe(
      JSON.stringify(expected, null, 2) + "\n"
    );
    expect(vi.mocked(runDafny).mock.calls.map((c) => [c[0], ...c[1]])).toEqual([
      [repo, "verify", "/work/proofs/Abs.dfy"],
      [repo, "audit", "/work/proofs/Abs.dfy"],
      [repo, "--version"],
    ]);
  });

  it("writes the same bytes on a second run (DE-11)", async () => {
    await mkdir(join(repo, "out"));
    const first = await dafnyEvidence({ ...input, outputPath: "out/a.json" });
    const second = await dafnyEvidence({ ...input, outputPath: "out/b.json" });
    expect(first.success && second.success).toBe(true);
    expect(await readFile(join(repo, "out", "a.json"), "utf-8")).toBe(
      await readFile(join(repo, "out", "b.json"), "utf-8")
    );
  });

  it("refuses a path that is not a git work tree (DE-2)", async () => {
    const plain = realpathSync(await mkdtemp(join(tmpdir(), "plain-")));
    try {
      const result = await dafnyEvidence({ ...input, repoPath: plain });
      expect(result.success).toBe(false);
      expect(result.errors).toEqual([`not a git work tree with a commit: ${plain}`]);
    } finally {
      await rm(plain, { recursive: true, force: true });
    }
  });

  it.each([
    ["an unstaged edit", async (r: string) => writeFile(join(r, "proofs", "Abs.dfy"), SOURCE + "\n"), " M proofs/Abs.dfy"],
    ["an untracked file", async (r: string) => writeFile(join(r, "proofs", "Extra.dfy"), ""), "?? proofs/Extra.dfy"],
  ])("refuses %s and names it (DE-3)", async (_label, dirty, line) => {
    await dirty(repo);
    const result = await dafnyEvidence(input);
    expect(result).toEqual({
      success: false,
      errors: [`work tree differs from ${commit}: ${line}`],
      record: null,
      writtenTo: null,
    });
    expect(runDafny).not.toHaveBeenCalled();
  });

  it("ignores ignored files (DE-3)", async () => {
    await mkdir(join(repo, "out"));
    await writeFile(join(repo, "out", "old.json"), "{}");
    expect((await dafnyEvidence(input)).success).toBe(true);
  });

  it("refuses a file that is not committed (DE-4)", async () => {
    const result = await dafnyEvidence({ ...input, file: "proofs/Missing.dfy" });
    expect(result.errors).toEqual(["not committed: proofs/Missing.dfy"]);
  });

  it("treats file as a literal path, not a glob (DE-4)", async () => {
    await writeFile(join(repo, "proofs", "Glob.dfy"), SOURCE);
    git(repo, "add", ".");
    git(repo, "commit", "-q", "-m", "second");
    const result = await dafnyEvidence({ ...input, file: "proofs/*.dfy" });
    expect(result.errors).toEqual(["not committed: proofs/*.dfy"]);
    expect(runDafny).not.toHaveBeenCalled();
  });

  it("refuses a tracked symbolic link as the file (DE-4)", async () => {
    await symlink("Abs.dfy", join(repo, "proofs", "Link.dfy"));
    git(repo, "add", ".");
    git(repo, "commit", "-q", "-m", "link");
    const result = await dafnyEvidence({ ...input, file: "proofs/Link.dfy" });
    expect(result.errors).toEqual(["proofs/Link.dfy is a symbolic link; pass the file it points to"]);
    expect(runDafny).not.toHaveBeenCalled();
  });

  it("still sees an untracked file when status.showUntrackedFiles is no (DE-3)", async () => {
    git(repo, "config", "status.showUntrackedFiles", "no");
    await writeFile(join(repo, "proofs", "Extra.dfy"), "");
    const result = await dafnyEvidence(input);
    expect(result.errors).toEqual([`work tree differs from ${commit}: ?? proofs/Extra.dfy`]);
  });

  it("refuses an audit that exits non-zero even with the clean text (DE-7)", async () => {
    stubDafny({ audit: { exitCode: 2, stdout: `${AUDIT_CLEAN}\n`, stderr: "", timedOut: false } });
    const result = await dafnyEvidence(input);
    expect(result.success).toBe(false);
    expect(result.errors[0]).toBe("dafny audit did not report 0 findings");
  });

  it("refuses an audit timeout even with the clean text (DE-7)", async () => {
    stubDafny({ audit: { exitCode: 0, stdout: `${AUDIT_CLEAN}\n`, stderr: "", timedOut: true } });
    const result = await dafnyEvidence(input);
    expect(result.success).toBe(false);
    expect(result.errors[0]).toBe("dafny audit did not report 0 findings");
  });

  it("refuses an undeclared theorem before running Dafny (DE-5)", async () => {
    const result = await dafnyEvidence({ ...input, theorems: ["AbsNonneg", "AbsPositive"] });
    expect(result.errors).toEqual(["theorem not declared in proofs/Abs.dfy: AbsPositive"]);
    expect(runDafny).not.toHaveBeenCalled();
  });

  it("names every dirty path (DE-3)", async () => {
    await writeFile(join(repo, "proofs", "A.dfy"), "");
    await writeFile(join(repo, "proofs", "Abs.dfy"), SOURCE + "\n");
    git(repo, "add", "proofs/A.dfy");
    const result = await dafnyEvidence(input);
    expect(result.errors).toEqual([
      `work tree differs from ${commit}: A  proofs/A.dfy`,
      `work tree differs from ${commit}:  M proofs/Abs.dfy`,
    ]);
  });

  it("refuses a repository with no commit (DE-2)", async () => {
    const bare = realpathSync(await mkdtemp(join(tmpdir(), "nocommit-")));
    try {
      git(bare, "init", "-q");
      const result = await dafnyEvidence({ ...input, repoPath: bare });
      expect(result.errors).toEqual([`not a git work tree with a commit: ${bare}`]);
    } finally {
      await rm(bare, { recursive: true, force: true });
    }
  });

  it("accepts the clean audit line on stderr (DE-7)", async () => {
    stubDafny({ audit: { exitCode: 0, stdout: "", stderr: `warnings first\n${AUDIT_CLEAN}\n`, timedOut: false } });
    expect((await dafnyEvidence(input)).success).toBe(true);
  });

  it("refuses an unreadable version whatever the exit code, and strict versions only (DE-8)", async () => {
    stubDafny({ "--version": { exitCode: 1, stdout: "4.11.0\n", stderr: "", timedOut: false } });
    expect((await dafnyEvidence(input)).success).toBe(false);
    for (const bad of ["4.11", "x4.11.0", "4.11.0 extra"]) {
      stubDafny({ "--version": ok(`${bad}\n`) });
      expect((await dafnyEvidence(input)).errors).toEqual([`could not read the Dafny version: ${bad}`]);
    }
  });

  it("reads the ID of the image it names (DE-8)", async () => {
    await dafnyEvidence(input);
    expect(dockerImageId).toHaveBeenCalledWith("crosscheck-dafny:latest");
  });

  it("refuses a failed verify (DE-6)", async () => {
    stubDafny({ verify: { exitCode: 4, stdout: "1 error", stderr: "warn on stderr", timedOut: false } });
    const result = await dafnyEvidence(input);
    expect(result.success).toBe(false);
    expect(result.errors).toEqual(["dafny verify exited 4", "1 error\nwarn on stderr"]);
    expect(result.record).toBeNull();
  });

  it("refuses a verify timeout (DE-6)", async () => {
    stubDafny({ verify: { exitCode: 0, stdout: "", stderr: "", timedOut: true } });
    expect((await dafnyEvidence(input)).errors[0]).toBe("dafny verify exited on timeout");
  });

  it("refuses an audit with findings even though it exits 0 (DE-7)", async () => {
    stubDafny({ audit: ok("Abs.dfy(1,15): Warning: Bad: Declaration has explicit `{:axiom}` attribute.\nDafny auditor completed with 1 findings\n") });
    const result = await dafnyEvidence(input);
    expect(result.success).toBe(false);
    expect(result.errors[0]).toBe("dafny audit did not report 0 findings");
    expect(result.errors[1]).toContain("{:axiom}");
  });

  it("refuses an unreadable Dafny version (DE-8)", async () => {
    stubDafny({ "--version": ok("dafny\n") });
    expect((await dafnyEvidence(input)).errors).toEqual(["could not read the Dafny version: dafny"]);
  });

  it("refuses when the image ID cannot be read (DE-8)", async () => {
    vi.mocked(dockerImageId).mockResolvedValue(null);
    expect((await dafnyEvidence(input)).errors).toEqual([
      "could not read the ID of image crosscheck-dafny:latest",
    ]);
  });

  it("returns the record but fails when the write fails (DE-11)", async () => {
    const result = await dafnyEvidence({ ...input, outputPath: "no/such/dir/r.json" });
    expect(result.success).toBe(false);
    expect(result.record?.commit).toBe(commit);
    expect(result.writtenTo).toBeNull();
    expect(result.errors[0]).toContain(`could not write ${join(repo, "no/such/dir/r.json")}`);
  });
});
