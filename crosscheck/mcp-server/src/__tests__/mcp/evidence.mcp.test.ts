import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { execFileSync } from "node:child_process";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { realpathSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";

vi.mock("../../docker.js", async (importOriginal) => ({
  SANDBOX_FLAGS: (await importOriginal<typeof import("../../docker.js")>()).SANDBOX_FLAGS,
  getDockerImage: vi.fn(() => "crosscheck-dafny:latest"),
  getLeanDockerImage: vi.fn(() => "crosscheck-lean:latest"),
  dockerImageId: vi.fn(async () => "sha256:feed"),
  runDafny: vi.fn(async (_dir: string, args: string[]) => ({
    exitCode: 0,
    stdout: {
      verify: "TestResult.DisplayName,TestResult.Outcome\nAbsNonneg (correctness),Passed,0,0,0\n",
      audit: "Dafny auditor completed with 0 findings\n",
      "--version": "4.11.0+fcb2042d\n",
    }[args[0]],
    stderr: "",
    timedOut: false,
  })),
  runLean: vi.fn(),
}));

import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { InMemoryTransport } from "@modelcontextprotocol/sdk/inMemory.js";
import { createServer } from "../../index.js";
import { rerunCommand } from "../../tools/evidence.js";

describe("dafny_evidence over MCP", () => {
  let client: Client;
  let repo: string;
  let commit: string;

  beforeEach(async () => {
    repo = realpathSync(await mkdtemp(join(tmpdir(), "evidence-mcp-")));
    const git = (...args: string[]) => execFileSync("git", ["-C", repo, ...args]).toString().trim();
    git("init", "-q");
    git("config", "user.email", "t@example.com");
    git("config", "user.name", "t");
    await writeFile(join(repo, "Abs.dfy"), "lemma AbsNonneg(x: int) ensures x * x >= 0 {}\n");
    git("add", ".");
    git("commit", "-q", "-m", "init");
    commit = git("rev-parse", "HEAD");

    const server = createServer();
    const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
    await server.connect(serverTransport);
    client = new Client({ name: "test-client", version: "1.0.0" });
    await client.connect(clientTransport);
  });

  afterEach(async () => {
    await client.close();
    await rm(repo, { recursive: true, force: true });
  });

  async function call(args: Record<string, unknown>) {
    const result = await client.callTool({ name: "dafny_evidence", arguments: args });
    const content = result.content as Array<{ type: string; text: string }>;
    return { isError: result.isError, body: JSON.parse(content[0].text) };
  }

  it.each([
    [null, null],
    [" docs/req.md#abs ", "docs/req.md#abs"],
  ])("passes every argument through and returns the record (requirement %j)", async (requirement, recorded) => {
    const { isError, body } = await call({
      repoPath: repo,
      file: "Abs.dfy",
      statement: "Squares are not negative.",
      requirement,
      theorems: ["AbsNonneg"],
      outputPath: "record.json",
    });
    expect(isError).toBeFalsy();
    expect(body).toEqual({
      success: true,
      errors: [],
      writtenTo: join(repo, "record.json"),
      record: {
        format: "evidence-record/1",
        commit,
        claims: [
          {
            id: "dafny-abs",
            statement: "Squares are not negative.",
            requirement: recorded,
            strength: "proved",
            basis: { theorems: ["AbsNonneg"] },
            trusted_base: [
              { component: "Dafny verifier", version: "4.11.0+fcb2042d" },
              { component: "Z3 solver shipped with the Dafny release", version: "Dafny 4.11.0+fcb2042d" },
              { component: "Dafny Docker image crosscheck-dafny:latest", version: "sha256:feed" },
            ],
            rerun: { command: rerunCommand("sha256:feed", ["Abs.dfy"]), exit_code: 0 },
          },
        ],
      },
    });
  });

  it("returns the tool's refusal as data", async () => {
    const { isError, body } = await call({
      repoPath: repo,
      file: "Abs.dfy",
      statement: "Squares are not negative.",
      requirement: null,
      theorems: ["M.AbsNonneg"],
    });
    expect(isError).toBeFalsy();
    expect(body).toEqual({
      success: false,
      errors: [
        "theorem not verified in Abs.dfy or its includes: M.AbsNonneg; name it as Dafny's verification log does, qualified by every enclosing module and type",
      ],
      record: null,
      writtenTo: null,
    });
  });
});
