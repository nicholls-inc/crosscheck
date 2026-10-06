import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";
import { dafnyVerify } from "./tools/verify.js";
import { dafnyCompile } from "./tools/compile.js";
import { dafnyCleanup } from "./tools/cleanup.js";
import { dafnyEvidence } from "./tools/evidence.js";
import { leanCheck } from "./tools/leanCheck.js";
import { leanRun } from "./tools/leanRun.js";
import { leanTest } from "./tools/leanTest.js";

export function createServer(): McpServer {
  const server = new McpServer({
    name: "crosscheck-dafny",
    version: "1.0.0",
  });

  server.tool(
    "dafny_verify",
    "Verify Dafny source code. Writes source to a temp file, runs `dafny verify`, and returns structured results with errors/warnings and difficulty metrics (solver time, resource count, proof hint count, trivial proof detection).",
    {
      source: z.string().describe("Dafny source code to verify"),
    },
    async ({ source }) => {
      const result = await dafnyVerify({ source });
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    }
  );

  server.tool(
    "dafny_evidence",
    "Emit an evidence record (evidence-record/1) with one `proved` claim for a committed Dafny file. Requires a clean git work tree. Runs `dafny verify` and `dafny audit` on the file as committed, checks each named theorem is declared in it, and refuses unless verification passes and the audit has 0 findings (an `{:axiom}` passes verify but not the audit). Returns { success, errors, record, writtenTo }. The record names the commit, the trusted base (Dafny version, its bundled Z3, the image ID) and a rerun command.",
    {
      repoPath: z.string().describe("Absolute path inside the git work tree"),
      file: z.string().describe("Path of the .dfy file relative to the work tree's top level, with / separators"),
      statement: z.string().describe("What the theorems prove, in plain language for a reader who will not open the code"),
      requirement: z
        .string()
        .nullable()
        .describe("Repository path (optionally #anchor) of the requirement the claim traces to, or null"),
      theorems: z
        .array(z.string())
        .describe("Names of the lemmas, methods or functions whose contracts prove the statement, optionally module-qualified as M.Name"),
      outputPath: z
        .string()
        .optional()
        .describe("Where to write the record; relative paths resolve against the work tree's top level"),
    },
    async (args) => {
      const result = await dafnyEvidence(args);
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    }
  );

  server.tool(
    "dafny_compile",
    "Compile verified Dafny source to Python or Go. Runs `dafny build`, strips Dafny runtime boilerplate, and returns clean output files.",
    {
      source: z.string().describe("Dafny source code to compile"),
      target: z
        .enum(["py", "go"])
        .describe("Target language: 'py' for Python, 'go' for Go"),
    },
    async ({ source, target }) => {
      const result = await dafnyCompile({ source, target });
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    }
  );

  server.tool(
    "dafny_cleanup",
    "Remove stale Dafny/Lean temp directories (older than 30 minutes) from /tmp.",
    {},
    async () => {
      const result = await dafnyCleanup();
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    }
  );

  server.tool(
    "lean_check",
    "Parse + typecheck Lean 4 source via `lake build` in the Mathlib-pre-warmed harness. Returns { success, kind: 'success' | 'parse-error' | 'typecheck-error' | 'build-error' | 'timeout', errors, warnings, sorries }. `sorry` warnings are expected for spec stubs and are surfaced separately from real warnings.",
    {
      source: z.string().describe("Lean 4 source code to typecheck"),
    },
    async ({ source }) => {
      const result = await leanCheck({ source });
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    }
  );

  server.tool(
    "lean_run",
    "Build + execute a Lean 4 file's `main : IO Unit` entry point. Used by /lean-impl for sanity-checking functional models against worked-example inputs, and by /drt-oracle as the Lean-side runner that the DRT harness invokes per random input. Not for spec stubs (which contain `sorry`).",
    {
      source: z.string().describe("Lean 4 source code with a `main : IO Unit` entry point"),
    },
    async ({ source }) => {
      const result = await leanRun({ source });
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    }
  );

  server.tool(
    "lean_test",
    "Run a Lean 4 test harness over a user module. The runner aliases this to `lake build`, which is sufficient for compile-time `#guard` and `decide` checks against literal fixtures. Sub-phase 3b-β chose not to wire a `lake test` driver: `/drt-oracle` invokes `lean_run` against per-def runners under `formal-verification/lean/CrosscheckModel/<Name>Runner.lean` driven by an external Python harness, which gives random-input fuzzing without coupling the MCP surface to a Lake test target. `lean_test` therefore remains the compile-time `#guard` path.",
    {
      source: z.string().describe("Lean 4 source code containing test declarations"),
    },
    async ({ source }) => {
      const result = await leanTest({ source });
      return {
        content: [{ type: "text" as const, text: JSON.stringify(result, null, 2) }],
      };
    }
  );

  return server;
}

async function main() {
  const server = createServer();
  const transport = new StdioServerTransport();
  await server.connect(transport);
}

main().catch((err) => {
  console.error("Fatal error:", err);
  process.exit(1);
});
