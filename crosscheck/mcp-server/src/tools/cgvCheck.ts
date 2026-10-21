import { execFile, type ExecFileException } from "node:child_process";
import { access, constants, readFile, rm, stat } from "node:fs/promises";
import { delimiter, isAbsolute, join } from "node:path";
import { z } from "zod";
import { git, treeChanges } from "../git.js";
import { createTempDir, removeTempDir } from "../tempdir.js";

// Spec: intent/2026-10-21-cgv-check-mcp-tool-spec.md (CK-1 to CK-13).

export const cgvCheckInputShape = {
  repoPath: z.string().describe("Absolute path inside the git work tree"),
  path: z
    .string()
    .describe("The Python project to check: a directory or a .py file, relative to the work tree's top level with / separators, or . for the top level"),
  baseline: z
    .string()
    .optional()
    .describe("A git revision, such as the pull request's base. When given, the findings are only those this revision's findings do not match"),
};

export type CgvCheckInput = z.infer<z.ZodObject<typeof cgvCheckInputShape>>;

type JsonObject = Record<string, unknown>;

export type CgvCheckOutput =
  | { success: false; errors: string[] }
  | {
      success: true;
      commit: string;
      baseline: string | null;
      exitCode: 0 | 1;
      findings: JsonObject;
      record: JsonObject | null;
    };

interface Binaries {
  cli: string;
  checker: string;
}

export const BUILD_COMMAND =
  "git clone https://github.com/nicholls-inc/crosscheck && cd crosscheck/cgv && cargo build --release && " +
  "(cd prover && lake build contract-graph-checker) && " +
  'echo "CGV_CONTRACTS=$PWD/target/release/crosscheck-contracts CGV_CHECKER=$PWD/prover/.lake/build/bin/contract-graph-checker"';

const HOW_TO_INSTALL =
  `Build both binaries with: ${BUILD_COMMAND} ` +
  "Then set CGV_CONTRACTS and CGV_CHECKER to the paths it prints in the environment Claude Code starts in, or put both binaries on PATH. " +
  "The build needs cargo (https://rustup.rs) and elan (https://github.com/leanprover/elan).";

const CGV_TIMEOUT_MS = 10 * 60 * 1000;
const CGV_MAX_BUFFER = 512 * 1024 * 1024;
const RECORD_FORMAT = "evidence-record/1";

function refuse(errors: string[]): CgvCheckOutput {
  return { success: false, errors };
}

export function validateCgvCheckInput(input: CgvCheckInput): string[] {
  const errors: string[] = [];
  for (const [name, value] of Object.entries(input)) {
    if (typeof value === "string" && value.includes("\0")) errors.push(`${name} must not contain a NUL byte`);
  }
  if (!isAbsolute(input.repoPath)) errors.push(`repoPath must be absolute: ${input.repoPath}`);
  if (
    input.path !== "." &&
    (input.path.includes("\\") || input.path.split("/").some((s) => s === "" || s === "." || s === ".."))
  ) {
    errors.push(`path must be . or a relative path with / separators and no empty, "." or ".." segment: ${input.path}`);
  }
  if (input.baseline !== undefined && !/^[^\s-]\S*$/.test(input.baseline)) {
    errors.push(`baseline must be a git revision that is not blank, holds no white space and does not start with "-": ${input.baseline}`);
  }
  return errors;
}

async function isExecutableFile(path: string): Promise<boolean> {
  try {
    if (!(await stat(path)).isFile()) return false;
    await access(path, constants.X_OK);
    return true;
  } catch {
    return false;
  }
}

async function findBinary(env: NodeJS.ProcessEnv, variable: string, name: string): Promise<string | { error: string }> {
  const named = env[variable];
  if (named !== undefined && named !== "") {
    if (isAbsolute(named) && (await isExecutableFile(named))) return named;
    return { error: `${variable} is set to ${named}, which is not an absolute path to an executable file. ${HOW_TO_INSTALL}` };
  }
  for (const dir of (env.PATH ?? "").split(delimiter)) {
    if (!isAbsolute(dir)) continue;
    const candidate = join(dir, name);
    if (await isExecutableFile(candidate)) return candidate;
  }
  return { error: `${name} not found: ${variable} is not set and no ${name} is on PATH. ${HOW_TO_INSTALL}` };
}

export async function findBinaries(env: NodeJS.ProcessEnv): Promise<Binaries | { errors: string[] }> {
  const cli = await findBinary(env, "CGV_CONTRACTS", "crosscheck-contracts");
  const checker = await findBinary(env, "CGV_CHECKER", "contract-graph-checker");
  if (typeof cli === "string" && typeof checker === "string") return { cli, checker };
  return { errors: [cli, checker].flatMap((b) => (typeof b === "string" ? [] : [b.error])) };
}

// Spec CK-4: a directory, or a regular .py file, committed at `rev`.
async function projectProblem(root: string, rev: string, path: string): Promise<string | null> {
  if (path === ".") return null;
  const entry = await git(root, ["ls-tree", "-z", rev, "--", path]);
  if (!entry.ok) return `git could not list ${path} at ${rev}`;
  const mode = /^(\d{6}) \S+ [0-9a-f]+\t/.exec(entry.stdout)?.[1];
  if (mode === undefined) return `not committed: ${path}`;
  if (mode === "040000") return null;
  if ((mode === "100644" || mode === "100755") && path.endsWith(".py")) return null;
  if (mode === "120000") return `${path} is a symbolic link at ${rev}; pass the directory or file it points to`;
  if (mode === "160000") return `${path} is a submodule at ${rev}`;
  return `${path} is neither a directory nor a .py file at ${rev}`;
}

interface Run {
  failure: string | null;
  exitCode: number | null;
  stdout: string;
  stderr: string;
}

function run(file: string, args: string[], cwd: string): Promise<Run> {
  const started = Date.now();
  return new Promise((done) => {
    execFile(file, args, { cwd, timeout: CGV_TIMEOUT_MS, maxBuffer: CGV_MAX_BUFFER }, (err: ExecFileException | null, stdout, stderr) => {
      let failure: string | null = null;
      let exitCode: number | null = 0;
      if (err !== null) {
        exitCode = typeof err.code === "number" ? err.code : null;
        if (exitCode !== null) failure = `exited ${exitCode}`;
        else if (err.killed && Date.now() - started >= CGV_TIMEOUT_MS) failure = "timed out after 10 minutes";
        else if (err.code === "ERR_CHILD_PROCESS_STDIO_MAXBUFFER") failure = "produced more output than the 512 MiB limit";
        else if (err.signal) failure = `was stopped by ${err.signal}`;
        else failure = `could not run: ${err.message}`;
      }
      done({ failure, exitCode, stdout, stderr });
    });
  });
}

function parseObject(text: string): JsonObject | null {
  try {
    const value: unknown = JSON.parse(text);
    return typeof value === "object" && value !== null && !Array.isArray(value) ? (value as JsonObject) : null;
  } catch {
    return null;
  }
}

function cgvFailed(what: string, ran: Run): string[] {
  return [`${what} ${ran.failure ?? `exited ${ran.exitCode}`}`, ran.stderr.trim()].filter((l) => l !== "");
}

const completed = (ran: Run): ran is Run & { exitCode: 0 | 1 } => ran.exitCode === 0 || ran.exitCode === 1;

// Spec CK-5: a checkout, not `git archive`, which applies .gitattributes export rules that HEAD's work tree does not.
async function writeBaseline(
  bins: Binaries,
  root: string,
  tmp: string,
  baseline: string,
  path: string
): Promise<string | string[]> {
  const tree = join(tmp, "baseline-tree");
  try {
    const added = await git(root, ["-c", "core.hooksPath=/dev/null", "worktree", "add", "--detach", tree, baseline]);
    if (!added.ok) return [`git could not check out ${baseline} in ${tree}`];
    const file = join(tmp, "baseline.json");
    const ran = await run(
      bins.cli,
      ["contracts", "check", join(tree, path), "--lean-checker", bins.checker, "--output-db", join(tmp, "baseline.sqlite"), "--write-baseline", file],
      tree
    );
    const what = `crosscheck-contracts at the baseline ${baseline}`;
    if (!completed(ran)) return cgvFailed(what, ran);
    if (!(await stat(file).catch(() => null))?.isFile()) return [`${what} exited ${ran.exitCode} and wrote no baseline file`];
    return file;
  } finally {
    // No `git worktree prune`: it is repository-wide and would also drop the entries of the caller's other worktrees whose
    // directories are missing. `remove --force` deregisters this checkout, and fails harmlessly when `add` registered nothing.
    await git(root, ["worktree", "remove", "--force", tree]);
    await rm(tree, { recursive: true, force: true }).catch(() => undefined);
  }
}

async function readRecord(file: string, commit: string): Promise<JsonObject | string> {
  const text = await readFile(file, "utf-8").catch(() => null);
  if (text === null) return "crosscheck-contracts exited 0 and wrote no evidence record";
  const record = parseObject(text);
  if (record === null) return "the evidence record crosscheck-contracts wrote is not a JSON object";
  if (record.format !== RECORD_FORMAT) return `the evidence record's format is not ${RECORD_FORMAT}: ${JSON.stringify(record.format)}`;
  if (record.commit !== commit) return `the evidence record names commit ${JSON.stringify(record.commit)}, not HEAD ${commit}`;
  return record;
}

async function runCgv(bins: Binaries, root: string, commit: string, tmp: string, input: CgvCheckInput, baseline: string | null): Promise<CgvCheckOutput> {
  let baselineFile: string | null = null;
  if (baseline !== null) {
    const written = await writeBaseline(bins, root, tmp, baseline, input.path);
    if (Array.isArray(written)) return refuse(written);
    baselineFile = written;
  }

  const project = join(root, input.path);
  const check = (db: string, extra: string[]) =>
    run(bins.cli, ["contracts", "check", project, "--lean-checker", bins.checker, "--output-db", join(tmp, db), ...extra], root);

  const full = await check("head.sqlite", ["--evidence-record", join(tmp, "record.json")]);
  if (!completed(full)) return refuse(cgvFailed(`crosscheck-contracts at ${commit}`, full));
  let findings = parseObject(full.stdout);
  if (findings === null) return refuse([`crosscheck-contracts at ${commit} exited ${full.exitCode} and printed no JSON object`]);
  let record: JsonObject | null = null;
  if (full.exitCode === 0) {
    const read = await readRecord(join(tmp, "record.json"), commit);
    if (typeof read === "string") return refuse([read]);
    record = read;
  }

  let exitCode = full.exitCode;
  if (baselineFile !== null) {
    const diff = await check("diff.sqlite", ["--baseline", baselineFile]);
    if (!completed(diff)) return refuse(cgvFailed(`crosscheck-contracts at ${commit} against the baseline ${baseline}`, diff));
    findings = parseObject(diff.stdout);
    if (findings === null) return refuse([`crosscheck-contracts at ${commit} against the baseline ${baseline} printed no JSON object`]);
    exitCode = diff.exitCode;
  }

  const headAfter = await git(root, ["rev-parse", "HEAD"]);
  if (!headAfter.ok || headAfter.stdout.trim() !== commit) {
    return refuse([`HEAD moved from ${commit} while CGV ran: ${headAfter.stdout.trim()}`]);
  }
  const dirtyAfter = await treeChanges(root);
  if (dirtyAfter === null) return refuse([`git could not read the work tree state in ${root}`]);
  if (dirtyAfter.length > 0) return refuse(dirtyAfter.map((l) => `work tree changed while CGV ran: ${l}`));

  return { success: true, commit, baseline, exitCode, findings, record };
}

export async function cgvCheck(input: CgvCheckInput, env: NodeJS.ProcessEnv = process.env): Promise<CgvCheckOutput> {
  const inputErrors = validateCgvCheckInput(input);
  if (inputErrors.length > 0) return refuse(inputErrors);
  const bins = await findBinaries(env);
  if ("errors" in bins) return refuse(bins.errors);

  const top = await git(input.repoPath, ["rev-parse", "--show-toplevel"]);
  const head = await git(input.repoPath, ["rev-parse", "HEAD"]);
  if (!top.ok || !head.ok) return refuse([`not a git work tree with a commit: ${input.repoPath}`]);
  const root = top.stdout.trim();
  const commit = head.stdout.trim();

  const dirty = await treeChanges(root);
  if (dirty === null) return refuse([`git could not read the work tree state in ${root}`]);
  if (dirty.length > 0) return refuse(dirty.map((l) => `work tree differs from ${commit}: ${l}`));
  const problem = await projectProblem(root, commit, input.path);
  if (problem !== null) return refuse([problem]);

  let baseline: string | null = null;
  if (input.baseline !== undefined) {
    const resolved = await git(root, ["rev-parse", "--verify", "--quiet", "--end-of-options", `${input.baseline}^{commit}`]);
    if (!resolved.ok) return refuse([`baseline is not a commit in ${root}: ${input.baseline}`]);
    baseline = resolved.stdout.trim();
    const atBaseline = await projectProblem(root, baseline, input.path);
    if (atBaseline !== null) return refuse([`at the baseline ${baseline}: ${atBaseline}`]);
  }

  let tmp: string;
  try {
    tmp = await createTempDir("cgv-");
  } catch (err) {
    return refuse([`could not create a temporary directory: ${(err as Error).message}`]);
  }
  try {
    return await runCgv(bins, root, commit, tmp, input, baseline);
  } catch (err) {
    return refuse([`cgv_check failed: ${(err as Error).message}`]);
  } finally {
    // A cleanup failure must not replace the result already computed (CK-10).
    await removeTempDir(tmp).catch(() => undefined);
  }
}
