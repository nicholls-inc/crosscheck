import { describe, it, expect, vi, afterEach } from "vitest";
import { rm } from "node:fs/promises";

vi.mock("../../docker.js", async (importOriginal) => {
  const real = await importOriginal<typeof import("../../docker.js")>();
  return { ...real, runDafny: vi.fn(real.runDafny), dockerImageId: vi.fn(real.dockerImageId) };
});

import { dockerImageId, runDafny } from "../../docker.js";
import { dafnyEvidence } from "../../tools/evidence.js";
import {
  PROGRAMS,
  commitProgram,
  dafnyOnly,
  readDafnyOutput,
  withoutDurations,
  writeDafnyOutput,
  type DafnyOutput,
} from "../fixtures/dafny-output.js";

// RECORD_DAFNY_FIXTURES=1 rewrites the fixtures that the integration test replays. Without it, the
// live output must match them, so an image whose Dafny prints something new fails here.
describe.skipIf(!process.env.RUN_E2E)("real Dafny output matches the recorded fixtures", () => {
  const repos: string[] = [];
  afterEach(async () => {
    for (const r of repos.splice(0)) await rm(r, { recursive: true, force: true });
  });

  it.each(PROGRAMS.map((p) => [p.name, p] as const))("%s", async (_name, program) => {
    const repo = await commitProgram(program, "dafny-output-e2e-");
    repos.push(repo);
    await dafnyEvidence({
      repoPath: repo,
      file: program.file,
      statement: "Recorded for the fixtures.",
      requirement: null,
      theorems: program.theorems,
    });
    const imageId = await vi.mocked(dockerImageId).mock.results[0].value;
    const runs = await Promise.all(
      vi.mocked(runDafny).mock.calls.map(async ([, args], i) => ({
        args,
        ...(await vi.mocked(runDafny).mock.results[i].value),
      }))
    );
    const live: DafnyOutput = { program, imageId, runs: runs.map(dafnyOnly) };
    if (process.env.RECORD_DAFNY_FIXTURES) await writeDafnyOutput(live);
    const recorded = await readDafnyOutput(program.name);
    expect({ ...live, runs: live.runs.map(withoutDurations) }).toEqual({
      ...recorded,
      runs: recorded.runs.map(withoutDurations),
    });
  }, 300_000);
});
