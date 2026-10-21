import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { chmod, mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { BUILD_COMMAND, findBinaries, validateCgvCheckInput } from "../../tools/cgvCheck.js";

describe("validateCgvCheckInput (CK-1)", () => {
  const ok = { repoPath: "/repo", path: "app" };

  it.each([
    [{ ...ok }],
    [{ ...ok, path: "." }],
    [{ ...ok, path: "src/app/models.py" }],
    [{ ...ok, baseline: "origin/main" }],
    [{ ...ok, baseline: "HEAD~1" }],
  ])("accepts %j", (input) => {
    expect(validateCgvCheckInput(input)).toEqual([]);
  });

  it.each([
    [{ ...ok, repoPath: "repo" }, "repoPath must be absolute: repo"],
    ...["", "/app", "app/", "./app", "a/../b", "a//b", "a\\b", ".."].map(
      (path) => [{ ...ok, path }, `path must be . or a relative path with / separators and no empty, "." or ".." segment: ${path}`] as const
    ),
    ...["", " ", "--output=x", "-x", "main extra", "a\tb"].map(
      (baseline) =>
        [{ ...ok, baseline }, `baseline must be a git revision that is not blank, holds no white space and does not start with "-": ${baseline}`] as const
    ),
  ])("refuses %j", (input, error) => {
    expect(validateCgvCheckInput(input)).toEqual([error]);
  });

  it("reports every broken rule", () => {
    expect(validateCgvCheckInput({ repoPath: "r", path: "../x", baseline: "-b" })).toHaveLength(3);
  });
});

describe("findBinaries (CK-2)", () => {
  let dir: string;

  beforeEach(async () => {
    dir = await mkdtemp(join(tmpdir(), "cgv-bins-"));
    await mkdir(join(dir, "bin"));
    await mkdir(join(dir, "other"));
  });

  afterEach(async () => {
    await rm(dir, { recursive: true, force: true });
  });

  async function executable(path: string, mode = 0o755) {
    await writeFile(path, "#!/bin/sh\n");
    await chmod(path, mode);
    return path;
  }

  it("takes each binary from its variable before PATH", async () => {
    const cli = await executable(join(dir, "other", "cgv-cli"));
    const checker = await executable(join(dir, "other", "cgv-checker"));
    await executable(join(dir, "bin", "crosscheck-contracts"));
    await executable(join(dir, "bin", "contract-graph-checker"));
    expect(await findBinaries({ CGV_CONTRACTS: cli, CGV_CHECKER: checker, PATH: join(dir, "bin") })).toEqual({ cli, checker });
  });

  it("falls back to the first match on PATH, skipping relative entries and non-executable files", async () => {
    await executable(join(dir, "other", "crosscheck-contracts"), 0o644);
    const cli = await executable(join(dir, "bin", "crosscheck-contracts"));
    const checker = await executable(join(dir, "bin", "contract-graph-checker"));
    const PATH = ["relative", join(dir, "other"), join(dir, "bin")].join(":");
    expect(await findBinaries({ PATH })).toEqual({ cli, checker });
  });

  it("refuses a variable that names no executable file, without falling back to PATH", async () => {
    await executable(join(dir, "bin", "crosscheck-contracts"));
    const checker = await executable(join(dir, "bin", "contract-graph-checker"));
    const plain = await executable(join(dir, "other", "plain"), 0o644);
    for (const named of [plain, join(dir, "missing"), "crosscheck-contracts", join(dir, "other")]) {
      const found = await findBinaries({ CGV_CONTRACTS: named, CGV_CHECKER: checker, PATH: join(dir, "bin") });
      expect(found).toEqual({
        errors: [expect.stringMatching(new RegExp(`^CGV_CONTRACTS is set to .*, which is not an absolute path to an executable file\\. Build both binaries with: `))],
      });
      expect((found as { errors: string[] }).errors[0]).toContain(BUILD_COMMAND);
    }
  });

  it("names each missing binary and gives the build command", async () => {
    const found = await findBinaries({ PATH: join(dir, "bin") });
    expect(found).toEqual({
      errors: [
        expect.stringMatching(/^crosscheck-contracts not found: CGV_CONTRACTS is not set and no crosscheck-contracts is on PATH\. /),
        expect.stringMatching(/^contract-graph-checker not found: CGV_CHECKER is not set and no contract-graph-checker is on PATH\. /),
      ],
    });
    for (const error of (found as { errors: string[] }).errors) {
      expect(error).toContain(`Build both binaries with: ${BUILD_COMMAND} Then set CGV_CONTRACTS and CGV_CHECKER`);
    }
  });

  it("builds both binaries from a fresh clone and prints both variables", () => {
    expect(BUILD_COMMAND).toBe(
      "git clone https://github.com/nicholls-inc/crosscheck && cd crosscheck/cgv && cargo build --release && " +
        "(cd prover && lake build contract-graph-checker) && " +
        'echo "CGV_CONTRACTS=$PWD/target/release/crosscheck-contracts CGV_CHECKER=$PWD/prover/.lake/build/bin/contract-graph-checker"'
    );
  });
});
