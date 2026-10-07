import { describe, it, expect, afterEach } from "vitest";
import { execFileSync, spawnSync } from "node:child_process";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { realpathSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { dockerImageId, getDockerImage } from "../../docker.js";
import { dafnyEvidence, rerunCommand } from "../../tools/evidence.js";

const repos: string[] = [];

async function imageId(): Promise<string> {
  const id = await dockerImageId(getDockerImage());
  if (id === null) throw new Error(`could not read the ID of image ${getDockerImage()}`);
  return id;
}

async function repoWith(source: string, extra: Record<string, string> = {}): Promise<string> {
  const repo = realpathSync(await mkdtemp(join(tmpdir(), "evidence-e2e-")));
  repos.push(repo);
  const git = (...args: string[]) => execFileSync("git", ["-C", repo, ...args]);
  git("init", "-q");
  git("config", "user.email", "t@example.com");
  git("config", "user.name", "t");
  await writeFile(join(repo, "Abs.dfy"), source);
  for (const [name, text] of Object.entries(extra)) await writeFile(join(repo, name), text);
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
    expect(result.record!.claims[0].rerun.command).toBe(rerunCommand(await imageId(), ["Abs.dfy"]));
    const rerun = spawnSync("sh", ["-c", result.record!.claims[0].rerun.command], { cwd: repo });
    expect(rerun.status).toBe(0);
  }, 300_000);

  it("takes a theorem in a module only by its fully qualified name", async () => {
    const repo = await repoWith("module M {\n  class C {\n    lemma L(x: int) ensures x * 1 == x {}\n  }\n}\n");
    const input = {
      repoPath: repo,
      file: "Abs.dfy",
      statement: "Multiplying by one changes nothing.",
      requirement: null,
    };
    expect((await dafnyEvidence({ ...input, theorems: ["M.C.L"] })).errors).toEqual([]);
    expect((await dafnyEvidence({ ...input, theorems: ["L", "M.L"] })).errors).toEqual([
      "theorem not verified in Abs.dfy or its includes: L; name it as Dafny's verification log does, qualified by every enclosing module and type",
      "theorem not verified in Abs.dfy or its includes: M.L; name it as Dafny's verification log does, qualified by every enclosing module and type",
    ]);
  }, 300_000);

  it("verifies and audits included files, and its rerun does too", async () => {
    const input = {
      file: "Abs.dfy",
      statement: "One is two.",
      requirement: null,
      theorems: ["T"],
    };
    const main = 'include "Lib.dfy"\nlemma T() ensures 1 == 2 { Bad(); }\n';

    const unproved = await repoWith(main, { "Lib.dfy": "lemma Bad() ensures false {}\n" });
    const r1 = await dafnyEvidence({ ...input, repoPath: unproved });
    expect(r1.errors[0]).toBe("dafny verify exited 4");
    const rerun1 = spawnSync("sh", ["-c", rerunCommand(await imageId(), ["Abs.dfy", "Lib.dfy"])], { cwd: unproved });
    expect(rerun1.status).toBe(4);

    const axiom = await repoWith(main, { "Lib.dfy": "lemma {:axiom} Bad() ensures false\n" });
    const r2 = await dafnyEvidence({ ...input, repoPath: axiom });
    expect(r2.errors[0]).toBe("dafny audit did not report 0 findings");
    expect(r2.errors[1]).toContain("Lib.dfy(1,15)");
    const rerun2 = spawnSync("sh", ["-c", rerunCommand(await imageId(), ["Abs.dfy", "Lib.dfy"])], { cwd: axiom });
    expect(rerun2.status).toBe(1);

    const proved = await repoWith('include "Lib.dfy"\nlemma T() ensures 2 == 2 { Ok(); }\n', {
      "Lib.dfy": "lemma Ok() ensures 1 == 1 {}\n",
    });
    const r3 = await dafnyEvidence({ ...input, repoPath: proved, theorems: ["T", "Ok"] });
    expect(r3.errors).toEqual([]);
    const rerun3 = spawnSync("sh", ["-c", r3.record!.claims[0].rerun.command], { cwd: proved });
    expect(rerun3.status).toBe(0);
  }, 600_000);

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
    const rerun = spawnSync("sh", ["-c", rerunCommand(await imageId(), ["Abs.dfy"])], { cwd: repo });
    expect(rerun.status).toBe(1);
  }, 300_000);
});
