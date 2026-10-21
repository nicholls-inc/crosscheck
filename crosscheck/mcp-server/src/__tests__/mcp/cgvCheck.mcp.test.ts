import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { execFileSync } from "node:child_process";
import { chmod, mkdir, mkdtemp, readFile, rm, symlink, writeFile } from "node:fs/promises";
import { existsSync, realpathSync } from "node:fs";
import { dirname, join } from "node:path";
import { tmpdir } from "node:os";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { InMemoryTransport } from "@modelcontextprotocol/sdk/inMemory.js";
import { createServer } from "../../index.js";
import { BUILD_COMMAND } from "../../tools/cgvCheck.js";

// A stand-in for crosscheck-contracts. FAKE_CGV holds, per run (baseline, full, diff), the exit code,
// stdout and stderr to give, and what to do while running. Each call is logged to FAKE_CGV_LOG.
const STAND_IN = `#!${process.execPath}
const { execFileSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");
const args = process.argv.slice(2);
const opt = (name) => { const i = args.indexOf(name); return i < 0 ? null : args[i + 1]; };
const kind = opt("--write-baseline") ? "baseline" : opt("--baseline") ? "diff" : "full";
const run = JSON.parse(process.env.FAKE_CGV)[kind];
const project = args[2];
const projectFiles = fs.statSync(project).isDirectory()
  ? Object.fromEntries(
      fs.readdirSync(project, { withFileTypes: true }).filter((e) => e.isFile()).map((e) => [e.name, fs.readFileSync(path.join(project, e.name), "utf-8")])
    )
  : null;
fs.appendFileSync(process.env.FAKE_CGV_LOG, JSON.stringify({ kind, cwd: process.cwd(), args, projectFiles }) + "\\n");
if (run.during === "commit") execFileSync("git", ["commit", "-q", "--allow-empty", "-m", "during"]);
if (run.during === "untracked") fs.writeFileSync("added.py", "");
if (kind === "baseline" && run.write !== false) fs.writeFileSync(opt("--write-baseline"), '{"format":"cgv-baseline/1","findings":[]}');
if (kind === "full" && run.record !== undefined) {
  const head = execFileSync("git", ["rev-parse", "HEAD"]).toString().trim();
  const record = { format: "evidence-record/1", commit: head, claims: [{ id: "cgv-data-paths" }], ...run.record };
  fs.writeFileSync(opt("--evidence-record"), typeof run.record === "string" ? run.record : JSON.stringify(record));
}
process.stderr.write(run.stderr ?? "");
process.stdout.write(run.stdout ?? JSON.stringify({ results: [{ kind }], exit_code: run.exit, counterexample: { kept: true } }));
process.exit(run.exit);
`;

interface Runs {
  baseline?: { exit: number; write?: boolean; stderr?: string; during?: string };
  full: { exit: number; record?: Record<string, unknown> | string; stdout?: string; stderr?: string; during?: string };
  diff?: { exit: number; stdout?: string; stderr?: string; during?: string };
}

describe("cgv_check over MCP", () => {
  let client: Client;
  let scratch: string;
  let repo: string;
  let bin: string;
  let log: string;
  let base: string;
  let head: string;

  const git = (...args: string[]) => execFileSync("git", ["-C", repo, ...args]).toString().trim();

  beforeEach(async () => {
    scratch = realpathSync(await mkdtemp(join(tmpdir(), "cgv-mcp-")));
    repo = join(scratch, "repo");
    bin = join(scratch, "bin");
    log = join(scratch, "log.jsonl");
    await mkdir(join(repo, "app"), { recursive: true });
    await mkdir(bin);
    await writeFile(join(bin, "crosscheck-contracts"), STAND_IN);
    await chmod(join(bin, "crosscheck-contracts"), 0o755);
    await writeFile(join(bin, "contract-graph-checker"), "");
    await chmod(join(bin, "contract-graph-checker"), 0o755);
    git("init", "-q");
    git("config", "user.email", "t@example.com");
    git("config", "user.name", "t");
    await writeFile(join(repo, "app", "models.py"), "OLD\n");
    git("add", ".");
    git("commit", "-q", "-m", "base");
    base = git("rev-parse", "HEAD");
    await writeFile(join(repo, "app", "models.py"), "NEW\n");
    await writeFile(join(repo, "app", "views.py"), "VIEWS\n");
    git("add", ".");
    git("commit", "-q", "-m", "change");
    head = git("rev-parse", "HEAD");

    vi.stubEnv("CGV_CONTRACTS", join(bin, "crosscheck-contracts"));
    vi.stubEnv("CGV_CHECKER", join(bin, "contract-graph-checker"));
    vi.stubEnv("FAKE_CGV_LOG", log);
    stubRuns({ full: { exit: 0, record: {} } });

    const server = createServer();
    const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
    await server.connect(serverTransport);
    client = new Client({ name: "test-client", version: "1.0.0" });
    await client.connect(clientTransport);
  });

  afterEach(async () => {
    vi.unstubAllEnvs();
    await client.close();
    await rm(scratch, { recursive: true, force: true });
  });

  function stubRuns(runs: Runs) {
    vi.stubEnv("FAKE_CGV", JSON.stringify(runs));
  }

  async function call(args: Record<string, unknown>) {
    const result = await client.callTool({ name: "cgv_check", arguments: { repoPath: repo, path: "app", ...args } });
    const content = result.content as Array<{ type: string; text: string }>;
    expect(result.isError).toBeFalsy();
    return JSON.parse(content[0].text);
  }

  async function calls(): Promise<Array<{ kind: string; cwd: string; args: string[]; projectFiles: Record<string, string> | null }>> {
    if (!existsSync(log)) return [];
    return (await readFile(log, "utf-8")).trim().split("\n").map((l) => JSON.parse(l));
  }

  describe("results (CK-9)", () => {
    it("returns CGV's findings, exit 0 and its record, from one run at HEAD in the work tree", async () => {
      const body = await call({});
      expect(body).toEqual({
        success: true,
        commit: head,
        baseline: null,
        exitCode: 0,
        findings: { results: [{ kind: "full" }], exit_code: 0, counterexample: { kept: true } },
        record: { format: "evidence-record/1", commit: head, claims: [{ id: "cgv-data-paths" }] },
      });
      const [run, ...rest] = await calls();
      expect(rest).toEqual([]);
      expect(run.cwd).toBe(repo);
      expect(run.args.slice(0, 5)).toEqual(["contracts", "check", join(repo, "app"), "--lean-checker", join(bin, "contract-graph-checker")]);
      expect(run.args).toContain("--evidence-record");
      expect(run.args).not.toContain("--baseline");
      expect(run.args).not.toContain("--allow-parse-errors");
    });

    it("checks the top level for path .", async () => {
      expect(await call({ path: "." })).toMatchObject({ success: true, exitCode: 0 });
      expect((await calls())[0].args[2]).toBe(repo);
    });

    it("returns no record when CGV exits 1", async () => {
      stubRuns({ full: { exit: 1 } });
      const body = await call({});
      expect(body).toMatchObject({ success: true, exitCode: 1, record: null, findings: { results: [{ kind: "full" }] } });
    });

    it("with a baseline, returns the comparison's findings and exit code, and the full run's record", async () => {
      stubRuns({ baseline: { exit: 1 }, full: { exit: 0, record: {} }, diff: { exit: 0 } });
      const body = await call({ baseline: "HEAD~1" });
      expect(body).toMatchObject({
        success: true,
        commit: head,
        baseline: base,
        exitCode: 0,
        findings: { results: [{ kind: "diff" }], exit_code: 0, counterexample: { kept: true } },
        record: { commit: head },
      });
      const [baselineRun, full, diff] = await calls();
      expect([baselineRun.kind, full.kind, diff.kind]).toEqual(["baseline", "full", "diff"]);
      expect(baselineRun.projectFiles).toEqual({ "models.py": "OLD\n" });
      expect(baselineRun.args[2].startsWith(repo)).toBe(false);
      const written = baselineRun.args[baselineRun.args.indexOf("--write-baseline") + 1];
      expect(diff.args[diff.args.indexOf("--baseline") + 1]).toBe(written);
      expect(diff.args).not.toContain("--evidence-record");
      expect(existsSync(dirname(written))).toBe(false);
    });

    it("checks the baseline in a checkout of it, so .gitattributes export rules do not change its files (CK-5)", async () => {
      await writeFile(join(repo, ".gitattributes"), "app/models.py export-ignore\napp/views.py export-subst\n");
      await writeFile(join(repo, "app", "views.py"), "$Format:%H$\n");
      git("add", ".");
      git("commit", "-q", "-m", "attributes");
      const attributed = git("rev-parse", "HEAD");
      await writeFile(join(repo, "README"), "x");
      git("add", ".");
      git("commit", "-q", "-m", "docs");
      stubRuns({ baseline: { exit: 0 }, full: { exit: 0, record: {} }, diff: { exit: 0 } });
      expect(await call({ baseline: attributed })).toMatchObject({ success: true, baseline: attributed });
      const [baselineRun] = await calls();
      expect(baselineRun.kind).toBe("baseline");
      expect(baselineRun.projectFiles).toEqual({ "models.py": "NEW\n", "views.py": "$Format:%H$\n" });
    });

    it.each([
      ["completes", { baseline: { exit: 0 }, full: { exit: 0, record: {} }, diff: { exit: 0 } }],
      ["fails", { baseline: { exit: 2, stderr: "boom" }, full: { exit: 0, record: {} } }],
    ])("removes the baseline checkout when the baseline run %s (CK-12)", async (_, runs) => {
      stubRuns(runs as Runs);
      await call({ baseline: base });
      const [baselineRun] = await calls();
      expect(git("worktree", "list", "--porcelain").split("\n").filter((l) => l.startsWith("worktree "))).toEqual([`worktree ${repo}`]);
      expect(existsSync(baselineRun.cwd)).toBe(false);
    });

    it("removes the baseline checkout when the baseline run leaves an untracked file in it, without pruning other worktrees (CK-12)", async () => {
      const elsewhere = join(scratch, "other-worktree");
      git("worktree", "add", "--detach", elsewhere, head);
      await rm(elsewhere, { recursive: true });
      stubRuns({ baseline: { exit: 0, during: "untracked" }, full: { exit: 0, record: {} }, diff: { exit: 0 } });
      expect(await call({ baseline: base })).toMatchObject({ success: true });
      const [baselineRun] = await calls();
      expect(existsSync(baselineRun.cwd)).toBe(false);
      const listed = git("worktree", "list", "--porcelain").split("\n").filter((l) => l.startsWith("worktree "));
      expect(listed).toEqual([`worktree ${repo}`, `worktree ${elsewhere}`]);
    });

    it("runs no hook of the repository on the baseline checkout (CK-5)", async () => {
      const marker = join(scratch, "hook-ran");
      const hook = join(repo, ".git", "hooks", "post-checkout");
      await writeFile(hook, `#!/bin/sh\ntouch "${marker}"\n`);
      await chmod(hook, 0o755);
      stubRuns({ baseline: { exit: 0 }, full: { exit: 0, record: {} }, diff: { exit: 0 } });
      expect(await call({ baseline: base })).toMatchObject({ success: true });
      expect(existsSync(marker)).toBe(false);
    });

    it("with a baseline, returns no record when the full run exits 1, even though no error is new", async () => {
      stubRuns({ baseline: { exit: 1 }, full: { exit: 1 }, diff: { exit: 0 } });
      expect(await call({ baseline: base })).toMatchObject({ success: true, exitCode: 0, record: null });
    });

    it("removes its temporary directory after a refusal", async () => {
      stubRuns({ full: { exit: 2, stderr: "boom" } });
      await call({});
      const run = (await calls())[0];
      expect(existsSync(dirname(run.args[run.args.indexOf("--output-db") + 1]))).toBe(false);
    });
  });

  describe("refusals (CK-10)", () => {
    const refused = (errors: unknown[]) => ({ success: false, errors });

    it("CK-1: an invalid input, before any run", async () => {
      expect(await call({ path: "../app" })).toEqual(
        refused([`path must be . or a relative path with / separators and no empty, "." or ".." segment: ../app`])
      );
      expect(await calls()).toEqual([]);
    });

    it("CK-2: missing binaries, with the build command", async () => {
      vi.stubEnv("CGV_CONTRACTS", "");
      vi.stubEnv("CGV_CHECKER", "");
      vi.stubEnv("PATH", join(scratch, "empty"));
      const body = await call({});
      expect(body.success).toBe(false);
      expect(body.errors).toHaveLength(2);
      for (const error of body.errors) expect(error).toContain(BUILD_COMMAND);
    });

    it("CK-3: a path that is not in a git work tree", async () => {
      expect(await call({ repoPath: bin })).toEqual(refused([`not a git work tree with a commit: ${bin}`]));
    });

    it("CK-3: a work tree that differs from HEAD anywhere", async () => {
      await writeFile(join(repo, "notes.txt"), "x");
      expect(await call({})).toEqual(refused([`work tree differs from ${head}: ?? notes.txt`]));
      expect(await calls()).toEqual([]);
    });

    it("CK-3: a file marked skip-worktree", async () => {
      git("update-index", "--skip-worktree", "app/views.py");
      expect(await call({})).toEqual(refused([`work tree differs from ${head}: skip-worktree hides changes to app/views.py`]));
    });

    it.each([
      ["missing", "not committed: missing"],
      ["app/views.txt", "not committed: app/views.txt"],
    ])("CK-4: path %s is refused", async (path, error) => {
      expect(await call({ path })).toEqual(refused([error]));
    });

    it("CK-4: a committed file that is not .py, and a symbolic link", async () => {
      await writeFile(join(repo, "README"), "x");
      await symlink("app", join(repo, "link"));
      git("add", ".");
      git("commit", "-q", "-m", "more");
      const now = git("rev-parse", "HEAD");
      expect(await call({ path: "README" })).toEqual(refused([`README is neither a directory nor a .py file at ${now}`]));
      expect(await call({ path: "link" })).toEqual(
        refused([`link is a symbolic link at ${now}; pass the directory or file it points to`])
      );
    });

    it("CK-5: a baseline that is not a commit", async () => {
      expect(await call({ baseline: "no-such-branch" })).toEqual(refused([`baseline is not a commit in ${repo}: no-such-branch`]));
    });

    it("CK-5: a path that does not exist at the baseline", async () => {
      expect(await call({ path: "app/views.py", baseline: base })).toEqual(
        refused([`at the baseline ${base}: not committed: app/views.py`])
      );
    });

    it("CK-5: a baseline run that does not complete", async () => {
      stubRuns({ baseline: { exit: 2, stderr: "Error: parse failed\n" }, full: { exit: 0, record: {} }, diff: { exit: 0 } });
      expect(await call({ baseline: base })).toEqual(
        refused([`crosscheck-contracts at the baseline ${base} exited 2`, "Error: parse failed"])
      );
      expect((await calls()).map((c) => c.kind)).toEqual(["baseline"]);
    });

    it("CK-5: a baseline run that writes no baseline file", async () => {
      stubRuns({ baseline: { exit: 0, write: false }, full: { exit: 0, record: {} }, diff: { exit: 0 } });
      expect(await call({ baseline: base })).toEqual(
        refused([`crosscheck-contracts at the baseline ${base} exited 0 and wrote no baseline file`])
      );
    });

    it("CK-6: a record run that CGV refuses", async () => {
      stubRuns({ full: { exit: 2, stdout: "", stderr: "Error: --evidence-record needs a clean checkout\n" } });
      expect(await call({})).toEqual(
        refused([`crosscheck-contracts at ${head} exited 2`, "Error: --evidence-record needs a clean checkout"])
      );
    });

    it("CK-6: output that is not a JSON object", async () => {
      stubRuns({ full: { exit: 1, stdout: "[]" } });
      expect(await call({})).toEqual(refused([`crosscheck-contracts at ${head} exited 1 and printed no JSON object`]));
    });

    it.each([
      [undefined, "crosscheck-contracts exited 0 and wrote no evidence record"],
      ["not json", "the evidence record crosscheck-contracts wrote is not a JSON object"],
      [{ format: "evidence-record/2" }, 'the evidence record\'s format is not evidence-record/1: "evidence-record/2"'],
      [{ commit: "0".repeat(40) }, `the evidence record names commit "${"0".repeat(40)}", not HEAD %HEAD%`],
    ])("CK-6: exit 0 with record %j", async (record, error) => {
      stubRuns({ full: { exit: 0, record } });
      expect(await call({})).toEqual(refused([error.replace("%HEAD%", head)]));
    });

    it("CK-7: a comparison run that does not complete", async () => {
      stubRuns({ baseline: { exit: 0 }, full: { exit: 0, record: {} }, diff: { exit: 2, stderr: "Error: cannot read the baseline\n" } });
      expect(await call({ baseline: base })).toEqual(
        refused([`crosscheck-contracts at ${head} against the baseline ${base} exited 2`, "Error: cannot read the baseline"])
      );
    });

    it("CK-8: HEAD moved while CGV ran", async () => {
      stubRuns({ baseline: { exit: 0 }, full: { exit: 0, record: {} }, diff: { exit: 0, during: "commit" } });
      const body = await call({ baseline: base });
      expect(body).toEqual(refused([`HEAD moved from ${head} while CGV ran: ${git("rev-parse", "HEAD")}`]));
    });

    it("CK-8: the work tree changed while CGV ran", async () => {
      stubRuns({ full: { exit: 1, during: "untracked" } });
      expect(await call({})).toEqual(refused(["work tree changed while CGV ran: ?? added.py"]));
    });
  });
});
