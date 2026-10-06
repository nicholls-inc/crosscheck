import { describe, it, expect, afterEach } from "vitest";
import { execFileSync, spawnSync } from "node:child_process";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { realpathSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { getDockerImage } from "../../docker.js";
import { dafnyEvidence, rerunCommand } from "../../tools/evidence.js";

const repos: string[] = [];

async function repoWith(source: string): Promise<string> {
  const repo = realpathSync(await mkdtemp(join(tmpdir(), "evidence-e2e-")));
  repos.push(repo);
  const git = (...args: string[]) => execFileSync("git", ["-C", repo, ...args]);
  git("init", "-q");
  git("config", "user.email", "t@example.com");
  git("config", "user.name", "t");
  await writeFile(join(repo, "Abs.dfy"), source);
  git("add", ".");
  git("commit", "-q", "-m", "init");
  return repo;
}

describe.skipIf(!process.env.RUN_E2E)("dafny_evidence E2E", () => {
  afterEach(async () => {
    for (const r of repos.splice(0)) await rm(r, { recursive: true, force: true });
  });

  it("emits a record whose rerun command exits 0", async () => {
    const repo = await repoWith(
      "function Abs(x: int): int { if x < 0 then -x else x }\nlemma AbsNonneg(x: int) ensures Abs(x) >= 0 {}\n"
    );
    const result = await dafnyEvidence({
      repoPath: repo,
      file: "Abs.dfy",
      statement: "Abs never returns a negative number.",
      requirement: null,
      theorems: ["AbsNonneg"],
    });
    expect(result.errors).toEqual([]);
    expect(result.record!.claims[0].trusted_base[0].version).toMatch(/^4\.11\.0/);
    const rerun = spawnSync("sh", ["-c", result.record!.claims[0].rerun.command], { cwd: repo });
    expect(rerun.status).toBe(0);
  }, 300_000);

  it("refuses an {:axiom} lemma that dafny verify accepts", async () => {
    const repo = await repoWith("lemma {:axiom} Bad(x: int) ensures x > 0\n");
    const result = await dafnyEvidence({
      repoPath: repo,
      file: "Abs.dfy",
      statement: "Every integer is positive.",
      requirement: null,
      theorems: ["Bad"],
    });
    expect(result.success).toBe(false);
    expect(result.errors[0]).toBe("dafny audit did not report 0 findings");
    const rerun = spawnSync("sh", ["-c", rerunCommand(getDockerImage(), "Abs.dfy")], { cwd: repo });
    expect(rerun.status).toBe(1);
  }, 300_000);
});
