import { execFileSync } from "node:child_process";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { realpathSync } from "node:fs";
import { dirname, join } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";

export interface Program {
  name: string;
  file: string;
  sources: Record<string, string>;
  theorems: string[];
}

export interface RecordedRun {
  args: string[];
  exitCode: number;
  stdout: string;
  stderr: string;
  timedOut: boolean;
}

export interface DafnyOutput {
  program: Program;
  imageId: string;
  runs: RecordedRun[];
}

const ABS = "function Abs(x: int): int { if x < 0 then -x else x }\nlemma AbsNonneg(x: int) ensures Abs(x) >= 0 {}\n";

export const PROGRAMS: Program[] = [
  { name: "proved", file: "Abs.dfy", sources: { "Abs.dfy": ABS }, theorems: ["AbsNonneg"] },
  {
    name: "qualified-name",
    file: "Abs.dfy",
    sources: { "Abs.dfy": "module M {\n  class C {\n    lemma L(x: int) ensures x * 1 == x {}\n  }\n}\n" },
    theorems: ["M.C.L"],
  },
  {
    name: "include-proved",
    file: "Abs.dfy",
    sources: {
      "Abs.dfy": 'include "Lib.dfy"\nlemma T() ensures 2 == 2 { Ok(); }\n',
      "Lib.dfy": "lemma Ok() ensures 1 == 1 {}\n",
    },
    theorems: ["T", "Ok"],
  },
  {
    name: "include-unproved",
    file: "Abs.dfy",
    sources: {
      "Abs.dfy": 'include "Lib.dfy"\nlemma T() ensures 1 == 2 { Bad(); }\n',
      "Lib.dfy": "lemma Bad() ensures false {}\n",
    },
    theorems: ["T"],
  },
  {
    name: "include-axiom",
    file: "Abs.dfy",
    sources: {
      "Abs.dfy": 'include "Lib.dfy"\nlemma T() ensures 1 == 2 { Bad(); }\n',
      "Lib.dfy": "lemma {:axiom} Bad() ensures false\n",
    },
    theorems: ["T"],
  },
  { name: "axiom", file: "Abs.dfy", sources: { "Abs.dfy": "lemma {:axiom} Bad(x: int) ensures x > 0\n" }, theorems: ["Bad"] },
  {
    name: "assume",
    file: "Abs.dfy",
    sources: { "Abs.dfy": "lemma Pos(x: int) ensures x > 0 { assume x > 0; }\n" },
    theorems: ["Pos"],
  },
  {
    name: "failing-postcondition",
    file: "Abs.dfy",
    sources: { "Abs.dfy": "lemma Neg(x: int) ensures x < 0 {}\n" },
    theorems: ["Neg"],
  },
];

export const FIXTURE_DIR = join(dirname(fileURLToPath(import.meta.url)), "dafny-4.11.0");

export async function readDafnyOutput(name: string): Promise<DafnyOutput> {
  return JSON.parse(await readFile(join(FIXTURE_DIR, `${name}.json`), "utf-8"));
}

export async function writeDafnyOutput(output: DafnyOutput): Promise<void> {
  await mkdir(FIXTURE_DIR, { recursive: true });
  await writeFile(join(FIXTURE_DIR, `${output.program.name}.json`), JSON.stringify(output, null, 2) + "\n");
}

// The Docker CLI prints this on a host whose platform differs from the image's. It is not Dafny's output.
const PLATFORM_WARNING = /^WARNING: The requested image's platform .*\n?/gm;

export function dafnyOnly(run: RecordedRun): RecordedRun {
  return { ...run, stdout: run.stdout.replace(PLATFORM_WARNING, ""), stderr: run.stderr.replace(PLATFORM_WARNING, "") };
}

// The third column of a verification log row is the time the proof took, which changes on every run.
export function withoutDurations(run: RecordedRun): RecordedRun {
  const strip = (s: string) => s.replace(/^([^,\n]+ \([a-z-]+\),[A-Za-z]+),[0-9:.]+,/gm, "$1,<duration>,");
  return { ...run, stdout: strip(run.stdout), stderr: strip(run.stderr) };
}

export async function commitProgram(program: Program, prefix: string): Promise<string> {
  const repo = realpathSync(await mkdtemp(join(tmpdir(), prefix)));
  const git = (...args: string[]) => execFileSync("git", ["-C", repo, ...args]);
  git("init", "-q");
  git("config", "user.email", "t@example.com");
  git("config", "user.name", "t");
  await writeFile(join(repo, ".gitignore"), "out/\n");
  for (const [path, text] of Object.entries(program.sources)) await writeFile(join(repo, path), text);
  git("add", ".");
  git("commit", "-q", "-m", "init");
  return repo;
}
