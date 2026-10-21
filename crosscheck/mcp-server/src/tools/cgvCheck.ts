import { execFile, type ExecFileException } from "node:child_process";
import { access, constants, mkdir, readFile, stat, writeFile } from "node:fs/promises";
import { delimiter, dirname, isAbsolute, join } from "node:path";
import { z } from "zod";
import { git, gitBytes, treeChanges } from "../git.js";
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

// Internal, not part of the MCP input schema: tests inject smaller limits to reach the refusals of CK-11.
export interface CgvLimits {
  timeoutMs: number;
  maxBuffer: number;
}

const MIB = 1024 * 1024;
export const DEFAULT_LIMITS: CgvLimits = { timeoutMs: 10 * 60 * 1000, maxBuffer: 512 * MIB };
export const TEMP_PREFIX = "cgv-check-";
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

const minutesOrSeconds = (ms: number) => (ms % 60000 === 0 ? `${ms / 60000} minutes` : `${ms / 1000} seconds`);
const mibOrBytes = (n: number) => (n % MIB === 0 ? `${n / MIB} MiB` : `${n} bytes`);

// Spec CK-11. The tool's own timer marks a timeout, because CGV handles SIGTERM and exits 143, which would read as an exit code.
function run(file: string, args: string[], cwd: string, limits: CgvLimits): Promise<Run> {
  return new Promise((done) => {
    let timedOut = false;
    const child = execFile(file, args, { cwd, maxBuffer: limits.maxBuffer }, (err: ExecFileException | null, stdout, stderr) => {
      clearTimeout(timer);
      let failure: string | null = null;
      let exitCode: number | null = typeof err?.code === "number" ? err.code : err === null ? 0 : null;
      if (timedOut) failure = `timed out after ${minutesOrSeconds(limits.timeoutMs)}`;
      else if (err?.code === "ERR_CHILD_PROCESS_STDIO_MAXBUFFER") failure = `produced more output than the ${mibOrBytes(limits.maxBuffer)} limit`;
      else if (err === null || exitCode !== null) failure = null;
      else if (err.signal) failure = `was stopped by ${err.signal}`;
      else failure = `could not run: ${err.message}`;
      if (failure !== null) exitCode = null;
      done({ failure, exitCode, stdout, stderr });
    });
    const timer = setTimeout(() => {
      timedOut = true;
      child.kill("SIGTERM");
    }, limits.timeoutMs);
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

// No lazy fetch from a promisor remote: building the baseline runs no transport or helper the repository's config names.
const NO_FETCH = { env: { ...process.env, GIT_NO_LAZY_FETCH: "1" } };

interface Blob {
  oid: string;
  path: string;
}

interface Tree {
  blobs: Blob[];
  refusals: string[];
}

// Spec CK-5: the regular files under `path` at `rev`, and a refusal for each entry the baseline tree cannot hold as committed.
async function listTree(root: string, rev: string, path: string): Promise<Tree | string> {
  const listed = await git(root, ["ls-tree", "-r", "-z", rev, ...(path === "." ? [] : ["--", path])], NO_FETCH);
  if (!listed.ok) return `git could not list ${path} at ${rev}`;
  const tree: Tree = { blobs: [], refusals: [] };
  for (const line of listed.stdout.split("\0").filter((l) => l !== "")) {
    const m = /^(\d{6}) \S+ ([0-9a-f]+)\t(.*)$/s.exec(line);
    if (m === null) return `git printed an entry cgv_check cannot read at ${rev}: ${JSON.stringify(line)}`;
    const [, mode, oid, entry] = m;
    if (entry.split("/").some((seg) => seg === "" || seg === "." || seg === "..")) tree.refusals.push(`${JSON.stringify(entry)} at ${rev} is not a plain relative path`);
    else if (mode === "100644" || mode === "100755") tree.blobs.push({ oid, path: entry });
    else if (mode === "120000") tree.refusals.push(`${entry} is a symbolic link at ${rev}; cgv_check writes no link into the baseline tree`);
    else if (mode === "160000") tree.refusals.push(`${entry} is a submodule at ${rev}; cgv_check cannot check a submodule`);
    else tree.refusals.push(`${entry} has mode ${mode} at ${rev}, which is not a regular file`);
  }
  return tree;
}

// Spec CK-5: a filter may make the checked-out bytes differ from the committed ones, so neither tree would be the committed project.
async function filteredPython(root: string, rev: string | null, blobs: Blob[]): Promise<string[]> {
  const py = blobs.map((b) => b.path).filter((p) => p.endsWith(".py"));
  if (py.length === 0) return [];
  const where = rev ?? "HEAD";
  const source = rev === null ? [] : ["--source", rev];
  const checked = await git(root, ["check-attr", "-z", "--stdin", ...source, "filter"], { ...NO_FETCH, input: py.join("\0") + "\0" });
  if (!checked.ok) return [`git could not read the filter attribute of the .py files at ${where}`];
  const fields = checked.stdout.split("\0");
  const errors: string[] = [];
  for (let i = 0; i + 2 < fields.length; i += 3) {
    const [file, , value] = fields.slice(i, i + 3);
    if (value !== "unspecified" && value !== "unset") {
      errors.push(`${file} has the git filter attribute "${value}" at ${where}, so its checked-out bytes may differ from the committed ones; cgv_check refuses a filtered .py file`);
    }
  }
  return errors;
}

// Spec CK-5: the baseline tree is the committed blobs, written as regular files with no checkout, filter, hook or helper.
async function buildTree(root: string, rev: string, blobs: Blob[], tree: string): Promise<string | null> {
  await mkdir(tree, { recursive: true });
  if (blobs.length === 0) return null;
  const batch = await gitBytes(root, ["cat-file", "--batch"], { ...NO_FETCH, input: blobs.map((b) => b.oid).join("\n") + "\n" });
  if (!batch.ok) return `git could not read the blobs of ${rev}`;
  let at = 0;
  for (const blob of blobs) {
    const eol = batch.stdout.indexOf(0x0a, at);
    const header = /^([0-9a-f]+) blob (\d+)$/.exec(batch.stdout.subarray(at, eol < 0 ? undefined : eol).toString());
    if (eol < 0 || header === null || header[1] !== blob.oid) return `git could not read ${blob.path} at ${rev}`;
    const size = Number(header[2]);
    const file = join(tree, blob.path);
    await mkdir(dirname(file), { recursive: true });
    await writeFile(file, batch.stdout.subarray(eol + 1, eol + 1 + size));
    at = eol + 1 + size + 1;
  }
  return null;
}

async function writeBaseline(
  bins: Binaries,
  root: string,
  tmp: string,
  baseline: string,
  blobs: Blob[],
  path: string,
  limits: CgvLimits
): Promise<string | string[]> {
  const tree = join(tmp, "baseline-tree");
  const built = await buildTree(root, baseline, blobs, tree);
  if (built !== null) return [built];
  const file = join(tmp, "baseline.json");
  const ran = await run(
    bins.cli,
    ["contracts", "check", join(tree, path), "--lean-checker", bins.checker, "--output-db", join(tmp, "baseline.sqlite"), "--write-baseline", file],
    tree,
    limits
  );
  const what = `crosscheck-contracts at the baseline ${baseline}`;
  if (!completed(ran)) return cgvFailed(what, ran);
  if (!(await stat(file).catch(() => null))?.isFile()) return [`${what} exited ${ran.exitCode} and wrote no baseline file`];
  return file;
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

interface Baseline {
  commit: string;
  blobs: Blob[];
}

async function runCgv(
  bins: Binaries,
  root: string,
  commit: string,
  tmp: string,
  input: CgvCheckInput,
  baseline: Baseline | null,
  limits: CgvLimits
): Promise<CgvCheckOutput> {
  let baselineFile: string | null = null;
  if (baseline !== null) {
    const written = await writeBaseline(bins, root, tmp, baseline.commit, baseline.blobs, input.path, limits);
    if (Array.isArray(written)) return refuse(written);
    baselineFile = written;
  }

  const project = join(root, input.path);
  const check = (db: string, extra: string[]) =>
    run(bins.cli, ["contracts", "check", project, "--lean-checker", bins.checker, "--output-db", join(tmp, db), ...extra], root, limits);

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
    const what = `crosscheck-contracts at ${commit} against the baseline ${baseline!.commit}`;
    if (!completed(diff)) return refuse(cgvFailed(what, diff));
    findings = parseObject(diff.stdout);
    if (findings === null) return refuse([`${what} printed no JSON object`]);
    exitCode = diff.exitCode;
  }

  const headAfter = await git(root, ["rev-parse", "HEAD"]);
  if (!headAfter.ok || headAfter.stdout.trim() !== commit) {
    return refuse([`HEAD moved from ${commit} while CGV ran: ${headAfter.stdout.trim()}`]);
  }
  const dirtyAfter = await treeChanges(root);
  if (dirtyAfter === null) return refuse([`git could not read the work tree state in ${root}`]);
  if (dirtyAfter.length > 0) return refuse(dirtyAfter.map((l) => `work tree changed while CGV ran: ${l}`));

  return { success: true, commit, baseline: baseline?.commit ?? null, exitCode, findings, record };
}

export async function cgvCheck(
  input: CgvCheckInput,
  env: NodeJS.ProcessEnv = process.env,
  limits: CgvLimits = DEFAULT_LIMITS
): Promise<CgvCheckOutput> {
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
  // At HEAD, CGV reads the work tree itself, so only the filter rule applies.
  const headTree = await listTree(root, commit, input.path);
  if (typeof headTree === "string") return refuse([headTree]);
  const headFiltered = await filteredPython(root, null, headTree.blobs);
  if (headFiltered.length > 0) return refuse(headFiltered);

  let baseline: Baseline | null = null;
  if (input.baseline !== undefined) {
    const resolved = await git(root, ["rev-parse", "--verify", "--quiet", "--end-of-options", `${input.baseline}^{commit}`]);
    if (!resolved.ok) return refuse([`baseline is not a commit in ${root}: ${input.baseline}`]);
    const at = resolved.stdout.trim();
    const atBaseline = await projectProblem(root, at, input.path);
    if (atBaseline !== null) return refuse([`at the baseline ${at}: ${atBaseline}`]);
    const tree = await listTree(root, at, input.path);
    if (typeof tree === "string") return refuse([tree]);
    if (tree.refusals.length > 0) return refuse(tree.refusals.map((e) => `at the baseline ${at}: ${e}`));
    const filtered = await filteredPython(root, at, tree.blobs);
    if (filtered.length > 0) return refuse(filtered);
    baseline = { commit: at, blobs: tree.blobs };
  }

  let tmp: string;
  try {
    tmp = await createTempDir(TEMP_PREFIX);
  } catch (err) {
    return refuse([`could not create a temporary directory: ${(err as Error).message}`]);
  }
  try {
    return await runCgv(bins, root, commit, tmp, input, baseline, limits);
  } catch (err) {
    return refuse([`cgv_check failed: ${(err as Error).message}`]);
  } finally {
    // A cleanup failure must not replace the result already computed (CK-10).
    await removeTempDir(tmp).catch(() => undefined);
  }
}
