// Runs locally only today: the mcp-server CI job does not build or receive the CGV binaries, so this file always skips in CI (ER-1.22 adds it).
// From the repository root:
// (cd cgv && cargo build --release && cd prover && lake build contract-graph-checker) && cd crosscheck/mcp-server && npx vitest run src/__tests__/integration/cgvCheck.cgv.integration.test.ts
import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { execFileSync, spawnSync } from "node:child_process";
import { cp, mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { realpathSync } from "node:fs";
import { delimiter, dirname, join } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { BUILD_COMMAND, cgvCheck, findBinaries } from "../../tools/cgvCheck.js";

const CGV = fileURLToPath(new URL("../../../../../cgv/", import.meta.url));
const FIXTURES = join(CGV, "test_fixtures");
const CHECKER = fileURLToPath(new URL("../../../../../scripts/check-evidence-record.mjs", import.meta.url));

// The binaries the tool would find, or else this checkout's own CGV build.
const env = {
  ...process.env,
  CGV_CONTRACTS: process.env.CGV_CONTRACTS || join(CGV, "target/release/crosscheck-contracts"),
  CGV_CHECKER: process.env.CGV_CHECKER || join(CGV, "prover/.lake/build/bin/contract-graph-checker"),
};
const binaries = await findBinaries(env);
const found = !("errors" in binaries);
const title = found
  ? "cgv_check on the real CGV binaries"
  : `cgv_check on the real CGV binaries: SKIPPED, no CGV binaries found. Set CGV_CONTRACTS and CGV_CHECKER, or build them in this checkout with: cd cgv && cargo build --release && (cd prover && lake build contract-graph-checker). From scratch: ${BUILD_COMMAND}`;
if (!found) console.warn(title);

describe.skipIf(!found)(title, () => {
  let repo: string;
  const git = (...args: string[]) => execFileSync("git", ["-C", repo, ...args]).toString().trim();

  async function commitFixture(name: string, message: string) {
    await cp(join(FIXTURES, name), join(repo, "app"), { recursive: true, filter: (src) => !src.endsWith("expected.json") });
    git("add", ".");
    git("commit", "-q", "-m", message);
    return git("rev-parse", "HEAD");
  }

  beforeEach(async () => {
    repo = realpathSync(await mkdtemp(join(tmpdir(), "cgv-integration-")));
    git("init", "-q");
    git("config", "user.email", "t@example.com");
    git("config", "user.name", "t");
    await mkdir(join(repo, "app"));
  });

  afterEach(async () => {
    await rm(repo, { recursive: true, force: true });
  });

  it("returns CGV's record for a project with no error, which the record checker accepts and whose rerun exits 0", async () => {
    const commit = await commitFixture("plain_python_clean", "clean");
    const result = await cgvCheck({ repoPath: repo, path: "app" }, env);
    expect(result).toMatchObject({ success: true, commit, baseline: null, exitCode: 0 });
    if (!result.success || result.record === null) throw new Error("expected a record");
    expect(result.findings).toHaveProperty("results");
    expect(result.record).toMatchObject({
      format: "evidence-record/1",
      commit,
      claims: [{ id: "cgv-data-paths", strength: "proved", basis: { theorems: ["ContractGraph.runChecker_sound_all"] } }],
    });

    const file = join(tmpdir(), `cgv-record-${process.pid}.json`);
    await writeFile(file, JSON.stringify(result.record));
    const check = spawnSync(process.execPath, [CHECKER, file], { encoding: "utf-8" });
    await rm(file);
    expect({ status: check.status, stdout: check.stdout }).toEqual({ status: 0, stdout: "" });

    const claim = (result.record.claims as Array<{ rerun: { command: string } }>)[0];
    const rerun = spawnSync("sh", ["-c", claim.rerun.command], {
      cwd: repo,
      encoding: "utf-8",
      env: { ...process.env, PATH: `${dirname(env.CGV_CONTRACTS)}${delimiter}${process.env.PATH}` },
    });
    expect(rerun.status).toBe(0);
  }, 120_000);

  it("returns exit 1 and no record for a project with an error", async () => {
    await commitFixture("bug1", "bug");
    const result = await cgvCheck({ repoPath: repo, path: "app" }, env);
    expect(result).toMatchObject({ success: true, exitCode: 1, record: null });
    if (!result.success) throw new Error("expected success");
    expect((result.findings.results as Array<{ severity: string }>).some((r) => r.severity === "error")).toBe(true);
  }, 120_000);

  it("with a baseline that holds the errors, returns exit 0, the existing count, and still no record", async () => {
    const base = await commitFixture("bug1", "bug");
    await writeFile(join(repo, "README.md"), "unrelated\n");
    git("add", ".");
    git("commit", "-q", "-m", "docs");
    const result = await cgvCheck({ repoPath: repo, path: "app", baseline: "HEAD~1" }, env);
    expect(result).toMatchObject({ success: true, baseline: base, exitCode: 0, record: null, findings: { results: [] } });
    if (!result.success) throw new Error("expected success");
    const counts = result.findings.baseline as { new: { errors: number }; existing: { errors: number } };
    expect(counts.new.errors).toBe(0);
    expect(counts.existing.errors).toBeGreaterThan(0);
  }, 120_000);

  it("with a baseline whose .gitattributes marks a .py file export-ignore, finds the errors a checkout of it holds", async () => {
    await writeFile(join(repo, ".gitattributes"), "app/models.py export-ignore\n");
    const base = await commitFixture("bug1", "bug");
    await writeFile(join(repo, "README.md"), "unrelated\n");
    git("add", ".");
    git("commit", "-q", "-m", "docs");
    const result = await cgvCheck({ repoPath: repo, path: "app", baseline: base }, env);
    expect(result).toMatchObject({ success: true, baseline: base, exitCode: 0, findings: { results: [] } });
    if (!result.success) throw new Error("expected success");
    const counts = result.findings.baseline as { new: { errors: number }; existing: { errors: number } };
    expect(counts.new.errors).toBe(0);
    expect(counts.existing.errors).toBeGreaterThan(0);
  }, 120_000);

  it("with a baseline that lacks the errors, returns exit 1 and the new errors", async () => {
    const base = await commitFixture("plain_python_clean", "clean");
    await commitFixture("bug1", "bug");
    const result = await cgvCheck({ repoPath: repo, path: "app", baseline: base }, env);
    expect(result).toMatchObject({ success: true, baseline: base, exitCode: 1, record: null });
    if (!result.success) throw new Error("expected success");
    expect((result.findings.baseline as { new: { errors: number } }).new.errors).toBeGreaterThan(0);
  }, 120_000);
});
