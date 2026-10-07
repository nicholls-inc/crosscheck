import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { execFileSync } from "node:child_process";
import { chmod, link, lstat, mkdtemp, mkdir, readFile, readdir, rename, rm, symlink, writeFile } from "node:fs/promises";
import { realpathSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";

vi.mock("node:fs/promises", async (original) => {
  const real = await original<typeof import("node:fs/promises")>();
  return { ...real, rename: vi.fn(real.rename) };
});

vi.mock("../../docker.js", () => ({
  getDockerImage: vi.fn(() => "crosscheck-dafny:latest"),
  dockerImageId: vi.fn(),
  runDafny: vi.fn(),
}));

import { dockerImageId, runDafny } from "../../docker.js";
import { dafnyEvidence, rerunCommand, type EvidenceInput } from "../../tools/evidence.js";

const AUDIT_CLEAN = "Dafny auditor completed with 0 findings";
const LOG_FORMAT = "csv;LogFileName=/dev/stdout";
const VERIFY_LOG = [
  "Dafny program verifier finished with 2 verified, 0 errors",
  "TestResult.DisplayName,TestResult.Outcome,TestResult.Duration,TestResult.ResourceCount,RandomSeed",
  "AbsNonneg (correctness),Passed,00:00:00.1,2473,0",
  "Arith.Twice (correctness),Passed,00:00:00.1,2473,0",
  "Results File: /dev/stdout",
].join("\n");

type Run = { exitCode: number; stdout: string; stderr: string; timedOut: boolean };
const ok = (stdout: string): Run => ({ exitCode: 0, stdout, stderr: "", timedOut: false });

function stubDafny(over: Partial<Record<"verify" | "audit" | "--version", Run>> = {}) {
  const runs: Record<string, Run> = {
    verify: ok(VERIFY_LOG),
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
          rerun: { command: rerunCommand("crosscheck-dafny:latest", ["proofs/Abs.dfy"]), exit_code: 0 },
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
    const run = { image: "sha256:feed", readOnly: true };
    expect(vi.mocked(runDafny).mock.calls).toEqual([
      [repo, ["verify", "/work/proofs/Abs.dfy", "--verify-included-files", "--log-format", LOG_FORMAT], run],
      [repo, ["audit", "/work/proofs/Abs.dfy"], run],
      [repo, ["--version"], run],
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

  it("refuses a glob as the file before git is asked about it (DE-1, DE-4)", async () => {
    await writeFile(join(repo, "proofs", "Glob.dfy"), SOURCE);
    git(repo, "add", ".");
    git(repo, "commit", "-q", "-m", "second");
    const result = await dafnyEvidence({ ...input, file: "proofs/*.dfy" });
    expect(result.errors).toEqual([
      'file must be a relative path to a .dfy file with no "." or ".." segment, made of A-Z a-z 0-9 _ . / - : proofs/*.dfy',
    ]);
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

  it("refuses a theorem the verification log does not report under that name (DE-5)", async () => {
    const result = await dafnyEvidence({ ...input, theorems: ["AbsNonneg", "Twice", "Arith.Twice", "AbsPositive"] });
    const hint = "name it as Dafny's verification log does, qualified by every enclosing module and type";
    expect(result).toEqual({
      success: false,
      errors: [
        `theorem not verified in proofs/Abs.dfy or its includes: Twice; ${hint}`,
        `theorem not verified in proofs/Abs.dfy or its includes: AbsPositive; ${hint}`,
      ],
      record: null,
      writtenTo: null,
    });
  });

  it("names an untracked file inside a new directory, not the directory (DE-3)", async () => {
    await mkdir(join(repo, "proofs", "new"));
    await writeFile(join(repo, "proofs", "new", "Deep.dfy"), "");
    const result = await dafnyEvidence(input);
    expect(result.errors).toEqual([`work tree differs from ${commit}: ?? proofs/new/Deep.dfy`]);
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

  it("reads the ID of the image it names before any Dafny run, and runs that ID (DE-8)", async () => {
    vi.mocked(runDafny).mockClear();
    await dafnyEvidence(input);
    expect(vi.mocked(dockerImageId).mock.calls).toEqual([["crosscheck-dafny:latest"]]);
    expect(vi.mocked(dockerImageId).mock.invocationCallOrder[0]).toBeLessThan(
      vi.mocked(runDafny).mock.invocationCallOrder[0]
    );
    expect(vi.mocked(runDafny).mock.calls.map((c) => c[2]?.image)).toEqual(["sha256:feed", "sha256:feed", "sha256:feed"]);
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

  it("refuses an audit whose clean text is only part of a line (DE-7)", async () => {
    stubDafny({ audit: ok(`Abs.dfy(1,15): Warning: ${AUDIT_CLEAN}\nDafny auditor completed with 1 findings\n`) });
    const result = await dafnyEvidence(input);
    expect(result.success).toBe(false);
    expect(result.errors[0]).toBe("dafny audit did not report 0 findings");
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
    expect(runDafny).not.toHaveBeenCalled();
  });

  it("returns the record but fails when the write fails, leaving the old record in place (DE-11)", async () => {
    const old = '{"format": "evidence-record/1"}\n';
    await mkdir(join(repo, "out"));
    await writeFile(join(repo, "out", "taken.json"), old);
    await chmod(join(repo, "out"), 0o555);
    try {
      const result = await dafnyEvidence({ ...input, outputPath: "out/taken.json" });
      expect(result.success).toBe(false);
      expect(result.record?.commit).toBe(commit);
      expect(result.writtenTo).toBeNull();
      expect(result.errors).toHaveLength(1);
      expect(result.errors[0]).toMatch(new RegExp(`^could not write ${join(repo, "out", "taken.json")}: EACCES`));
      expect(await readFile(join(repo, "out", "taken.json"), "utf-8")).toBe(old);
      expect(await readdir(join(repo, "out"))).toEqual(["taken.json"]);
    } finally {
      await chmod(join(repo, "out"), 0o755);
    }
  });

  it("checks outputPath again just before writing, so a link planted during the runs is refused (DE-11)", async () => {
    await mkdir(join(repo, "out"));
    vi.mocked(runDafny).mockImplementation(async (_dir, args) => {
      if (args[0] === "audit") await symlink(join(repo, "proofs", "Abs.dfy"), join(repo, "out", "r.json"));
      return args[0] === "--version" ? ok("4.11.0\n") : args[0] === "audit" ? ok(AUDIT_CLEAN) : ok(VERIFY_LOG);
    });
    const result = await dafnyEvidence({ ...input, outputPath: "out/r.json" });
    expect(result.success).toBe(false);
    expect(result.errors).toEqual(["outputPath out/r.json is a symbolic link"]);
    expect(result.record?.commit).toBe(commit);
    expect(await readFile(join(repo, "proofs", "Abs.dfy"), "utf-8")).toBe(SOURCE);
  });

  it.each([
    ["assume-unchanged", "--assume-unchanged"],
    ["skip-worktree", "--skip-worktree"],
  ])("refuses a tracked file whose %s flag hides an edit (DE-3)", async (flag, option) => {
    git(repo, "update-index", option, "proofs/Abs.dfy");
    await writeFile(join(repo, "proofs", "Abs.dfy"), SOURCE + "lemma {:axiom} Hidden() ensures false\n");
    const result = await dafnyEvidence(input);
    expect(result).toEqual({
      success: false,
      errors: [`work tree differs from ${commit}: ${flag} hides changes to proofs/Abs.dfy`],
      record: null,
      writtenTo: null,
    });
    expect(runDafny).not.toHaveBeenCalled();
  });

  // A `git` on PATH that fails the given subcommand, always or only once `marker` exists.
  async function withFailingGit(
    subcommand: string,
    marker: string | null,
    body: () => Promise<void>,
    action = "exit 1"
  ) {
    const bin = realpathSync(await mkdtemp(join(tmpdir(), "fake-git-")));
    const realGit = execFileSync("sh", ["-c", "command -v git"]).toString().trim();
    const guard = marker === null ? "" : `[ -e '${marker}' ] && `;
    await writeFile(
      join(bin, "git"),
      `#!/bin/sh\ncase " $* " in *" ${subcommand} "*) ${guard}${action} ;; esac\nexec ${realGit} "$@"\n`
    );
    await chmod(join(bin, "git"), 0o755);
    const path = process.env.PATH;
    process.env.PATH = `${bin}:${path}`;
    try {
      await body();
    } finally {
      process.env.PATH = path;
      await rm(bin, { recursive: true, force: true });
    }
  }

  it.each([["ls-files -v"], ["status --porcelain --untracked-files=all"]])(
    "refuses when git cannot run `%s` (DE-3)",
    async (subcommand) => {
      await withFailingGit(subcommand, null, async () => {
        expect((await dafnyEvidence(input)).errors).toEqual([`git could not read the work tree state in ${repo}`]);
      });
      expect(runDafny).not.toHaveBeenCalled();
    }
  );

  it("refuses when git cannot read the work tree state after the runs (DE-13)", async () => {
    const marker = join(repo, "..", `after-${process.pid}-${Date.now()}`);
    await withFailingGit("status --porcelain --untracked-files=all", marker, async () => {
      vi.mocked(runDafny).mockImplementation(async (_dir, args) => {
        if (args[0] === "audit") await writeFile(marker, "");
        return args[0] === "--version" ? ok("4.11.0\n") : args[0] === "audit" ? ok(AUDIT_CLEAN) : ok(VERIFY_LOG);
      });
      try {
        expect((await dafnyEvidence(input)).errors).toEqual([`git could not read the work tree state in ${repo}`]);
        expect(vi.mocked(runDafny).mock.calls.map((c) => c[1][0])).toEqual(["verify", "audit", "--version"]);
      } finally {
        await rm(marker, { force: true });
      }
    });
  });

  it("refuses to write through a file planted at the temporary name, and writes nothing else (DE-11)", async () => {
    await mkdir(join(repo, "out"));
    await writeFile(join(repo, "out", "Target.txt"), "keep");
    const temp = join(repo, "out", `r.json.${process.pid}.tmp`);
    await symlink(join(repo, "out", "Target.txt"), temp);
    const result = await dafnyEvidence({ ...input, outputPath: "out/r.json" });
    expect(result.success).toBe(false);
    expect(result.errors[0]).toMatch(/^could not write .*r\.json: EEXIST/);
    expect(await readFile(join(repo, "out", "Target.txt"), "utf-8")).toBe("keep");
    expect((await lstat(temp)).isSymbolicLink()).toBe(true);
  });

  it("reads `git ls-files -v` output past the 1 MiB default buffer (DE-3)", async () => {
    await withFailingGit(
      "ls-files -v",
      null,
      async () => {
        expect((await dafnyEvidence(input)).errors).toEqual([]);
      },
      "yes 'H proofs/Abs.dfy' | head -c 2000000; exit 0"
    );
  });

  it("leaves only the record in the output directory after a write (DE-11)", async () => {
    await mkdir(join(repo, "out"));
    expect((await dafnyEvidence({ ...input, outputPath: "out/r.json" })).success).toBe(true);
    expect(await readdir(join(repo, "out"))).toEqual(["r.json"]);
  });

  it("removes its temporary file when the rename fails (DE-11)", async () => {
    await mkdir(join(repo, "out"));
    vi.mocked(rename).mockRejectedValueOnce(new Error("EBUSY: rename refused"));
    const result = await dafnyEvidence({ ...input, outputPath: "out/r.json" });
    expect(result.success).toBe(false);
    expect(result.errors[0]).toMatch(/^could not write .*r\.json: EBUSY: rename refused$/);
    expect(await readdir(join(repo, "out"))).toEqual([]);
  });

  it("refuses when a flag hides a change made while Dafny runs (DE-13)", async () => {
    vi.mocked(runDafny).mockImplementation(async (_dir, args) => {
      if (args[0] === "verify") git(repo, "update-index", "--assume-unchanged", "proofs/Abs.dfy");
      return args[0] === "--version" ? ok("4.11.0\n") : args[0] === "audit" ? ok(AUDIT_CLEAN) : ok(VERIFY_LOG);
    });
    expect((await dafnyEvidence(input)).errors).toEqual([
      "work tree changed while Dafny ran: assume-unchanged hides changes to proofs/Abs.dfy",
    ]);
  });

  it("accepts a theorem declared in an include, by the name the log gives it (DE-5)", async () => {
    await commitFiles({ "proofs/Abs.dfy": `include "Lib.dfy"\n${SOURCE}`, "proofs/Lib.dfy": "" });
    vi.mocked(runDafny).mockImplementation(async (_dir, args) =>
      args[0] === "--version"
        ? ok("4.11.0\n")
        : args[0] === "audit"
          ? ok(AUDIT_CLEAN)
          : ok(`${VERIFY_LOG}\nLib.Helper (correctness),Passed,0,0,0`)
    );
    const result = await dafnyEvidence({ ...input, theorems: ["AbsNonneg", "Lib.Helper"] });
    expect(result.errors).toEqual([]);
    expect(result.record?.claims[0].basis).toEqual({ theorems: ["AbsNonneg", "Lib.Helper"] });
  });

  describe("outputPath containment (DE-11)", () => {
    async function refusesBeforeDafny(outputPath: string, why: string) {
      const result = await dafnyEvidence({ ...input, outputPath });
      expect(result).toEqual({
        success: false,
        errors: [`outputPath ${outputPath} ${why}`],
        record: null,
        writtenTo: null,
      });
      expect(runDafny).not.toHaveBeenCalled();
    }

    it("refuses a relative path that leaves the work tree", async () => {
      await refusesBeforeDafny("../escape.json", `is not inside the work tree ${repo}`);
    });

    it("refuses an absolute path outside the work tree", async () => {
      await refusesBeforeDafny(join(tmpdir(), "escape.json"), `is not inside the work tree ${repo}`);
    });

    it("refuses the work tree itself", async () => {
      await refusesBeforeDafny(".", `is not inside the work tree ${repo}`);
    });

    it("refuses a path inside .git, in any case", async () => {
      await refusesBeforeDafny(".git/hooks/pre-commit", "is inside .git");
      await refusesBeforeDafny("sub/.Git/config", "is inside .git");
    });

    it("refuses a missing directory before running Dafny", async () => {
      await refusesBeforeDafny("no/such/dir/r.json", `names a directory that does not exist: ${join(repo, "no/such/dir")}`);
    });

    it("refuses a directory reached through a symbolic link", async () => {
      const outside = realpathSync(await mkdtemp(join(tmpdir(), "outside-")));
      try {
        await mkdir(join(repo, "out"));
        await symlink(outside, join(repo, "out", "ext"));
        await refusesBeforeDafny("out/ext/r.json", "passes through a symbolic link");
      } finally {
        await rm(outside, { recursive: true, force: true });
      }
    });

    it("refuses an existing symbolic link as the output file", async () => {
      await mkdir(join(repo, "out"));
      await symlink(join(repo, "proofs", "Abs.dfy"), join(repo, "out", "r.json"));
      await refusesBeforeDafny("out/r.json", "is a symbolic link");
    });

    const NOT_A_RECORD = "names an existing file that is not an evidence record";

    it("refuses to overwrite the verified file, under any spelling", async () => {
      await refusesBeforeDafny("proofs/Abs.dfy", NOT_A_RECORD);
      await refusesBeforeDafny("proofs/../proofs/Abs.dfy", NOT_A_RECORD);
    });

    it("refuses to overwrite a hard link to the verified file", async () => {
      await mkdir(join(repo, "out"));
      await link(join(repo, "proofs", "Abs.dfy"), join(repo, "out", "r.json"));
      await refusesBeforeDafny("out/r.json", NOT_A_RECORD);
    });

    it("refuses to overwrite a file the verified file includes", async () => {
      await commitFiles({
        "proofs/Abs.dfy": `include "Lib.dfy"\n${SOURCE}`,
        "proofs/Lib.dfy": "lemma Lib() ensures true {}\n",
      });
      await refusesBeforeDafny("proofs/Lib.dfy", NOT_A_RECORD);
    });

    it("refuses to overwrite any other tracked or ignored file, or a directory", async () => {
      await commitFiles({ "docs/SKILL.md": "# skill\n", "docs/other.json": '{"format": "x"}\n' });
      await mkdir(join(repo, "out", "dir"), { recursive: true });
      await writeFile(join(repo, "out", ".env"), "TOKEN=1\n");
      await refusesBeforeDafny("docs/SKILL.md", NOT_A_RECORD);
      await refusesBeforeDafny("docs/other.json", NOT_A_RECORD);
      await refusesBeforeDafny("out/.env", NOT_A_RECORD);
      await refusesBeforeDafny("out/dir", NOT_A_RECORD);
    });

    it("refuses a FIFO without reading it", async () => {
      await mkdir(join(repo, "out"));
      execFileSync("mkfifo", [join(repo, "out", "pipe.json")]);
      await refusesBeforeDafny("out/pipe.json", NOT_A_RECORD);
    });

    it("overwrites an earlier record inside the tree", async () => {
      await commitFiles({ "evidence/abs.json": '{"format": "evidence-record/1", "commit": "old"}\n' });
      const result = await dafnyEvidence({ ...input, outputPath: "evidence/abs.json" });
      expect(result.writtenTo).toBe(join(repo, "evidence", "abs.json"));
      expect(await readFile(join(repo, "evidence", "abs.json"), "utf-8")).toBe(
        JSON.stringify(result.record, null, 2) + "\n"
      );
      expect(result.record?.commit).toBe(git(repo, "rev-parse", "HEAD"));
    });
  });

  describe("includes (DE-12)", () => {
    async function refusesBeforeDafny(errors: string[]) {
      const result = await dafnyEvidence(input);
      expect(result).toEqual({ success: false, errors, record: null, writtenTo: null });
      expect(runDafny).not.toHaveBeenCalled();
    }

    it("accepts tracked includes, nested and repeated", async () => {
      await commitFiles({
        "proofs/Abs.dfy": `include "lib/A.dfy"\ninclude "lib/B.dfy"\n${SOURCE}`,
        "proofs/lib/A.dfy": `include "B.dfy"\n`,
        "proofs/lib/B.dfy": `include "../lib/A.dfy"\n`,
      });
      const result = await dafnyEvidence(input);
      expect(result.errors).toEqual([]);
      expect(vi.mocked(runDafny).mock.calls[1][1]).toEqual([
        "audit",
        "/work/proofs/Abs.dfy",
        "/work/proofs/lib/A.dfy",
        "/work/proofs/lib/B.dfy",
      ]);
      expect(result.record?.claims[0].rerun.command).toBe(
        rerunCommand("crosscheck-dafny:latest", ["proofs/Abs.dfy", "proofs/lib/A.dfy", "proofs/lib/B.dfy"])
      );
    });

    it("checks an include inside a comment as well", async () => {
      await commitFiles({ "proofs/Abs.dfy": `// include "Gone.dfy"\n${SOURCE}` });
      await refusesBeforeDafny([
        'include "Gone.dfy" in proofs/Abs.dfy is outside the tracked files: not committed: proofs/Gone.dfy',
      ]);
    });

    it("refuses an include of an ignored file", async () => {
      await commitFiles({ "proofs/Abs.dfy": `include "../out/Gen.dfy"\n${SOURCE}` });
      await mkdir(join(repo, "out"));
      await writeFile(join(repo, "out", "Gen.dfy"), "lemma {:axiom} Gen()\n");
      await refusesBeforeDafny([
        'include "../out/Gen.dfy" in proofs/Abs.dfy is outside the tracked files: not committed: out/Gen.dfy',
      ]);
    });

    it("refuses an ignored file included by a tracked include", async () => {
      await commitFiles({
        "proofs/Abs.dfy": `include "Lib.dfy"\n${SOURCE}`,
        "proofs/Lib.dfy": `include "../out/Gen.dfy"\n`,
      });
      await refusesBeforeDafny([
        'include "../out/Gen.dfy" in proofs/Lib.dfy is outside the tracked files: not committed: out/Gen.dfy',
      ]);
    });

    it("refuses a tracked symbolic link as an include", async () => {
      await symlink("Abs.dfy", join(repo, "proofs", "Link.dfy"));
      await commitFiles({ "proofs/Abs.dfy": `include "Link.dfy"\n${SOURCE}` });
      await refusesBeforeDafny([
        'include "Link.dfy" in proofs/Abs.dfy is outside the tracked files: proofs/Link.dfy is a symbolic link; pass the file it points to',
      ]);
    });

    it("refuses an include reached through a symbolic link to a directory", async () => {
      await mkdir(join(repo, "real"));
      await writeFile(join(repo, "real", "Lib.dfy"), "");
      await symlink("real", join(repo, "alias"));
      await commitFiles({ "proofs/Abs.dfy": `include "../alias/Lib.dfy"\n${SOURCE}` });
      await refusesBeforeDafny([
        'include "../alias/Lib.dfy" in proofs/Abs.dfy is outside the tracked files: not committed: alias/Lib.dfy',
      ]);
    });

    it("refuses an include that names a tracked directory", async () => {
      await commitFiles({ "proofs/Abs.dfy": `include "lib"\n${SOURCE}`, "proofs/lib/A.dfy": "" });
      await refusesBeforeDafny([
        'include "lib" in proofs/Abs.dfy is outside the tracked files: proofs/lib is not a regular file',
      ]);
    });

    it.each([
      ["a verbatim string", 'include @"Lib.dfy"'],
      ["a block comment before the string", 'include /* c */ "Lib.dfy"'],
      ["a line comment before the string", 'include // c\n"Lib.dfy"'],
    ])("refuses an include written with %s, which Dafny reads but the scan cannot resolve", async (_name, line) => {
      await commitFiles({ "proofs/Abs.dfy": `${line}\n${SOURCE}`, "proofs/Lib.dfy": "" });
      await refusesBeforeDafny([
        "include in proofs/Abs.dfy is not followed by a plain \"<path>\" string, so it cannot be checked",
      ]);
    });

    it.each([
      ["a percent escape that Dafny decodes", "L%69b.dfy"],
      ["an escaped parent directory", "%2e%2e/Lib.dfy"],
      ["a trailing space, which Dafny trims", "Lib.dfy "],
    ])("refuses an include path with %s", async (_name, path) => {
      await commitFiles({ "proofs/Abs.dfy": `include "${path}"\n${SOURCE}`, "proofs/Lib.dfy": "" });
      await refusesBeforeDafny([
        `include "${path}" in proofs/Abs.dfy is outside the tracked files: the path has characters outside A-Z a-z 0-9 _ . / -, which Dafny may decode before it opens the file`,
      ]);
    });

    it.each([
      ["UTF-16 with a byte-order mark", Buffer.concat([Buffer.from([0xff, 0xfe]), Buffer.from('include "Lib.dfy"\n', "utf16le")])],
      ["UTF-16 big-endian with a mark", Buffer.concat([Buffer.from([0xfe, 0xff]), Buffer.from('include "Lib.dfy"\n', "utf16le").swap16()])],
      ["a NUL byte", Buffer.from('include "Lib.dfy"\n\0')],
    ])("refuses an included file that is %s, which Dafny decodes and the scan cannot read", async (_name, bytes) => {
      await commitFiles({ "proofs/Abs.dfy": `include "Wide.dfy"\n${SOURCE}`, "proofs/Wide.dfy": bytes, "proofs/Lib.dfy": "" });
      await refusesBeforeDafny([
        'include "Wide.dfy" in proofs/Abs.dfy is outside the tracked files: could not be read: proofs/Wide.dfy is not UTF-8 text (it has a NUL byte, as UTF-16 and UTF-32 text does)',
      ]);
    });

    it("refuses a verified file that is UTF-16 text", async () => {
      await commitFiles({ "proofs/Abs.dfy": Buffer.concat([Buffer.from([0xff, 0xfe]), Buffer.from(SOURCE, "utf16le")]) });
      await refusesBeforeDafny([
        "could not read proofs/Abs.dfy: proofs/Abs.dfy is not UTF-8 text (it has a NUL byte, as UTF-16 and UTF-32 text does)",
      ]);
    });

    it("accepts a UTF-8 byte-order mark and still scans the text after it", async () => {
      await commitFiles({ "proofs/Abs.dfy": `\uFEFFinclude "Lib.dfy"\n${SOURCE}`, "proofs/Lib.dfy": "" });
      expect((await dafnyEvidence(input)).errors).toEqual([]);
      vi.mocked(runDafny).mockClear();
      await commitFiles({ "proofs/Abs.dfy": `\uFEFFinclude "Missing.dfy"\n${SOURCE}` });
      await refusesBeforeDafny([
        'include "Missing.dfy" in proofs/Abs.dfy is outside the tracked files: not committed: proofs/Missing.dfy',
      ]);
    });

    it("applies the path allowlist to an include inside an included file", async () => {
      await commitFiles({ "proofs/Abs.dfy": `include "Mid.dfy"\n${SOURCE}`, "proofs/Mid.dfy": 'include "L%69b.dfy"\n' });
      await refusesBeforeDafny([
        'include "L%69b.dfy" in proofs/Mid.dfy is outside the tracked files: the path has characters outside A-Z a-z 0-9 _ . / -, which Dafny may decode before it opens the file',
      ]);
    });

    it("refuses a backslash in an include path with the outside-the-tree reason", async () => {
      await commitFiles({ "proofs/Abs.dfy": `include "..\\Lib.dfy"\n${SOURCE}` });
      await refusesBeforeDafny([
        'include "..\\Lib.dfy" in proofs/Abs.dfy is outside the tracked files: resolves outside the work tree: ..\\Lib.dfy',
      ]);
    });

    it("refuses an include outside the work tree, relative or absolute", async () => {
      await commitFiles({
        "proofs/Abs.dfy": `include "../../Up.dfy"\ninclude "/work/proofs/Abs.dfy"\ninclude "/Abs.dfy"\n${SOURCE}`,
      });
      await refusesBeforeDafny([
        'include "../../Up.dfy" in proofs/Abs.dfy is outside the tracked files: resolves outside the work tree: ../../Up.dfy',
        'include "/work/proofs/Abs.dfy" in proofs/Abs.dfy is outside the tracked files: resolves outside the work tree: /work/proofs/Abs.dfy',
        'include "/Abs.dfy" in proofs/Abs.dfy is outside the tracked files: resolves outside the work tree: /Abs.dfy',
      ]);
    });
  });

  describe("state after the Dafny runs (DE-13)", () => {
    it("refuses when HEAD moves while Dafny runs", async () => {
      vi.mocked(runDafny).mockImplementation(async (_dir, args) => {
        if (args[0] === "audit") git(repo, "commit", "-q", "--allow-empty", "-m", "during");
        return args[0] === "--version" ? ok("4.11.0\n") : args[0] === "audit" ? ok(AUDIT_CLEAN) : ok(VERIFY_LOG);
      });
      const result = await dafnyEvidence(input);
      expect(result).toEqual({
        success: false,
        errors: [`HEAD moved from ${commit} while Dafny ran: ${git(repo, "rev-parse", "HEAD")}`],
        record: null,
        writtenTo: null,
      });
    });

    it("refuses when the work tree changes while Dafny runs", async () => {
      vi.mocked(runDafny).mockImplementation(async (_dir, args) => {
        if (args[0] === "verify") await writeFile(join(repo, "proofs", "Abs.dfy"), SOURCE + "// edited\n");
        return args[0] === "--version" ? ok("4.11.0\n") : args[0] === "audit" ? ok(AUDIT_CLEAN) : ok(VERIFY_LOG);
      });
      const result = await dafnyEvidence(input);
      expect(result).toEqual({
        success: false,
        errors: ["work tree changed while Dafny ran:  M proofs/Abs.dfy"],
        record: null,
        writtenTo: null,
      });
    });
  });

  async function commitFiles(files: Record<string, string | Buffer>) {
    for (const [path, content] of Object.entries(files)) {
      await mkdir(join(repo, path, ".."), { recursive: true });
      await writeFile(join(repo, path), content);
    }
    git(repo, "add", ".");
    git(repo, "commit", "-q", "-m", "files");
  }
});
