import { execFile } from "node:child_process";
import { lstat, readFile, realpath, rename, unlink, writeFile } from "node:fs/promises";
import { dirname, isAbsolute, join, posix, relative, resolve as resolvePath, sep } from "node:path";
import { dockerImageId, getDockerImage, runDafny } from "../docker.js";

export interface EvidenceInput {
  repoPath: string;
  file: string;
  statement: string;
  requirement: string | null;
  theorems: string[];
  outputPath?: string;
}

interface TrustedComponent {
  component: string;
  version: string;
}

interface ProvedClaim {
  id: string;
  statement: string;
  requirement: string | null;
  strength: "proved";
  basis: { theorems: string[] };
  trusted_base: TrustedComponent[];
  rerun: { command: string; exit_code: number };
}

export interface EvidenceRecord {
  format: "evidence-record/1";
  commit: string;
  claims: [ProvedClaim];
}

export interface EvidenceOutput {
  success: boolean;
  errors: string[];
  record: EvidenceRecord | null;
  writtenTo: string | null;
}

// Dafny decodes a source file by its byte-order mark and percent-decodes a path, on the command line as
// well as in an include, so the scan reads only text it decodes the same way: no NUL byte (which
// UTF-16 and UTF-32 text always has), and `file` and every include path made of plain path characters.
const PLAIN_PATH = /^[A-Za-z0-9_./-]+$/;

const NAME = /^[A-Za-z_][A-Za-z0-9_'?]*(\.[A-Za-z_][A-Za-z0-9_'?]*)*$/;
const AUDIT_CLEAN = "Dafny auditor completed with 0 findings";

export function validateEvidenceInput(input: EvidenceInput): string[] {
  const errors: string[] = [];
  if (!isAbsolute(input.repoPath)) errors.push(`repoPath must be absolute: ${input.repoPath}`);
  const segments = input.file.split("/");
  if (
    isAbsolute(input.file) ||
    input.file.includes("\\") ||
    segments.some((s) => s === ".." || s === "." || s === "") ||
    !input.file.endsWith(".dfy") ||
    !PLAIN_PATH.test(input.file)
  ) {
    errors.push(
      `file must be a relative path to a .dfy file with no "." or ".." segment, made of A-Z a-z 0-9 _ . / - : ${input.file}`
    );
  }
  if (input.statement.trim() === "") errors.push("statement is blank");
  if (input.requirement !== null && input.requirement.trim() === "") {
    errors.push("requirement is blank; pass null when the claim traces to no requirement");
  }
  if (input.theorems.length === 0) errors.push("theorems is empty");
  for (const name of input.theorems) {
    if (!NAME.test(name)) errors.push(`not a Dafny name: ${name}`);
  }
  return errors;
}

const LOG_HEADER = "TestResult.DisplayName,";

export function verifiedNames(verifyStdout: string): Set<string> {
  const lines = verifyStdout.split(/\r?\n/);
  const header = lines.findIndex((l) => l.startsWith(LOG_HEADER));
  const names = new Set<string>();
  if (header < 0) return names;
  for (const line of lines.slice(header + 1)) {
    const match = /^(\S+) \([a-z-]+\),Passed,/.exec(line);
    if (match) names.add(match[1]);
  }
  return names;
}

export function unverifiedTheorems(verifyStdout: string, theorems: string[]): string[] {
  const names = verifiedNames(verifyStdout);
  return theorems.filter((t) => !names.has(t));
}

export function shellQuote(s: string): string {
  return `'${s.replace(/'/g, `'\\''`)}'`;
}

export function rerunCommand(image: string, files: string[]): string {
  const run = `docker run --rm --network=none -v "$PWD":/work:ro ${shellQuote(image)}`;
  const paths = files.map((f) => shellQuote(`/work/${f}`));
  return (
    `${run} verify ${paths[0]} --verify-included-files && out=$(${run} audit ${paths.join(" ")} 2>&1) && ` +
    `case "$out" in *'${AUDIT_CLEAN}'*) true ;; *) false ;; esac`
  );
}

export function claimId(file: string): string {
  const slug = file
    .replace(/\.dfy$/, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return slug === "" ? "dafny" : `dafny-${slug}`;
}

export function buildRecord(facts: {
  commit: string;
  file: string;
  includes: string[];
  statement: string;
  requirement: string | null;
  theorems: string[];
  dafnyVersion: string;
  image: string;
  imageId: string;
}): EvidenceRecord {
  return {
    format: "evidence-record/1",
    commit: facts.commit,
    claims: [
      {
        id: claimId(facts.file),
        statement: facts.statement.trim(),
        requirement: facts.requirement === null ? null : facts.requirement.trim(),
        strength: "proved",
        basis: { theorems: [...facts.theorems] },
        trusted_base: [
          { component: "Dafny verifier", version: facts.dafnyVersion },
          { component: "Z3 solver shipped with the Dafny release", version: `Dafny ${facts.dafnyVersion}` },
          { component: `Dafny Docker image ${facts.image}`, version: facts.imageId },
        ],
        rerun: { command: rerunCommand(facts.image, [facts.file, ...facts.includes]), exit_code: 0 },
      },
    ],
  };
}

// `git ls-files -v` prints a line per tracked file, so the 1 MiB default would refuse a large repository.
const GIT_MAX_BUFFER = 512 * 1024 * 1024;

function git(cwd: string, args: string[]): Promise<{ ok: boolean; stdout: string }> {
  return new Promise((done) => {
    execFile("git", ["--literal-pathspecs", "-C", cwd, ...args], { maxBuffer: GIT_MAX_BUFFER }, (err, stdout) => {
      done({ ok: err === null, stdout: String(stdout) });
    });
  });
}

async function treeChanges(root: string): Promise<string[] | null> {
  const status = await git(root, ["status", "--porcelain", "--untracked-files=all"]);
  const tags = await git(root, ["ls-files", "-v"]);
  if (!status.ok || !tags.ok) return null;
  const hidden = tags.stdout
    .split("\n")
    .filter((l) => /^[a-zS] /.test(l))
    .map((l) => `${/^[sS]/.test(l) ? "skip-worktree" : "assume-unchanged"} hides changes to ${l.slice(2)}`);
  return [...status.stdout.split("\n").filter((l) => l !== ""), ...hidden];
}

async function untrackedReason(root: string, path: string): Promise<string | null> {
  if (!(await git(root, ["ls-files", "--error-unmatch", "--", path])).ok) return `not committed: ${path}`;
  const stats = await lstat(resolvePath(root, path));
  if (stats.isSymbolicLink()) return `${path} is a symbolic link; pass the file it points to`;
  if (!stats.isFile()) return `${path} is not a regular file`;
  return null;
}

// Every `include` keyword is matched, with or without the plain string the scan can resolve. Dafny
// also reads `include @"x.dfy"` and `include /* c */ "x.dfy"`, so an `include` followed by anything
// else is refused rather than skipped.
const INCLUDE = /\binclude\b(?:\s+"([^"]*)")?/g;

async function readScannedSource(root: string, path: string): Promise<string> {
  const bytes = await readFile(resolvePath(root, path));
  // UTF-16 and UTF-32 text carries a NUL byte beside every ASCII character, `include` included.
  if (bytes.includes(0)) throw new Error(`${path} is not UTF-8 text (it has a NUL byte, as UTF-16 and UTF-32 text does)`);
  return bytes.toString("utf-8");
}

async function includedFiles(
  root: string,
  file: string,
  source: string
): Promise<{ files: string[]; errors: string[] }> {
  const files = [file];
  const errors: string[] = [];
  const queue: Array<[string, string]> = [[file, source]];
  while (queue.length > 0) {
    const [from, text] = queue.shift()!;
    for (const [, target] of text.matchAll(INCLUDE)) {
      if (target === undefined) {
        errors.push(`include in ${from} is not followed by a plain "<path>" string, so it cannot be checked`);
        continue;
      }
      const path = posix.normalize(posix.join(posix.dirname(from), target));
      const outside = isAbsolute(target) || target.includes("\\") || path === ".." || path.startsWith("../");
      if (!outside && PLAIN_PATH.test(target) && files.includes(path)) continue;
      let reason: string | null = `resolves outside the work tree: ${target}`;
      if (!outside && !PLAIN_PATH.test(target)) {
        reason = "the path has characters outside A-Z a-z 0-9 _ . / -, which Dafny may decode before it opens the file";
      } else if (!outside) {
        try {
          reason = await untrackedReason(root, path);
          if (reason === null) queue.push([path, await readScannedSource(root, path)]);
        } catch (err) {
          reason = `could not be read: ${(err as Error).message}`;
        }
      }
      if (reason === null) files.push(path);
      else errors.push(`include "${target}" in ${from} is outside the tracked files: ${reason}`);
    }
  }
  return { files, errors };
}

async function outputTarget(
  root: string,
  outputPath: string
): Promise<{ out: string; error: null } | { out: null; error: string }> {
  const fail = (why: string) => ({ out: null, error: `outputPath ${outputPath} ${why}` });
  const out = resolvePath(root, outputPath);
  const rel = relative(root, out);
  if (rel === "" || rel === ".." || rel.startsWith(`..${sep}`)) {
    return fail(`is not inside the work tree ${root}`);
  }
  if (rel.split(sep).some((s) => s.toLowerCase() === ".git")) return fail("is inside .git");
  let parent: string;
  try {
    parent = await realpath(dirname(out));
  } catch {
    return fail(`names a directory that does not exist: ${dirname(out)}`);
  }
  if (parent !== join(await realpath(root), dirname(rel))) return fail("passes through a symbolic link");
  const existing = await lstat(out).catch(() => null);
  if (existing === null) return { out, error: null };
  if (existing.isSymbolicLink()) return fail("is a symbolic link");
  if (!existing.isFile() || !(await isEvidenceRecord(out))) {
    return fail("names an existing file that is not an evidence record");
  }
  return { out, error: null };
}

async function isEvidenceRecord(path: string): Promise<boolean> {
  try {
    return JSON.parse(await readFile(path, "utf-8"))?.format === "evidence-record/1";
  } catch {
    return false;
  }
}

function refuse(errors: string[], record: EvidenceRecord | null = null): EvidenceOutput {
  return { success: false, errors, record, writtenTo: null };
}

export async function dafnyEvidence(input: EvidenceInput): Promise<EvidenceOutput> {
  const inputErrors = validateEvidenceInput(input);
  if (inputErrors.length > 0) return refuse(inputErrors);

  const top = await git(input.repoPath, ["rev-parse", "--show-toplevel"]);
  const head = await git(input.repoPath, ["rev-parse", "HEAD"]);
  if (!top.ok || !head.ok) return refuse([`not a git work tree with a commit: ${input.repoPath}`]);
  const root = top.stdout.trim();
  const commit = head.stdout.trim();

  const dirty = await treeChanges(root);
  if (dirty === null) return refuse([`git could not read the work tree state in ${root}`]);
  if (dirty.length > 0) return refuse(dirty.map((l) => `work tree differs from ${commit}: ${l}`));

  let source: string;
  try {
    const reason = await untrackedReason(root, input.file);
    if (reason !== null) return refuse([reason]);
    source = await readScannedSource(root, input.file);
  } catch (err) {
    return refuse([`could not read ${input.file}: ${(err as Error).message}`]);
  }
  const included = await includedFiles(root, input.file, source);
  if (included.errors.length > 0) return refuse(included.errors);

  let out: string | null = null;
  if (input.outputPath !== undefined) {
    const target = await outputTarget(root, input.outputPath);
    if (target.error !== null) return refuse([target.error]);
    out = target.out;
  }

  const image = getDockerImage();
  const imageId = await dockerImageId(image);
  if (imageId === null) return refuse([`could not read the ID of image ${image}`]);

  const run = { image: imageId, readOnly: true };
  const paths = included.files.map((f) => `/work/${f}`);
  const verify = await runDafny(
    root,
    ["verify", paths[0], "--verify-included-files", "--log-format", "csv;LogFileName=/dev/stdout"],
    run
  );
  if (verify.timedOut || verify.exitCode !== 0) {
    return refuse([
      `dafny verify exited ${verify.timedOut ? "on timeout" : verify.exitCode}`,
      (verify.stdout + "\n" + verify.stderr).trim(),
    ]);
  }
  const audit = await runDafny(root, ["audit", ...paths], run);
  const auditOutput = audit.stdout + "\n" + audit.stderr;
  if (audit.timedOut || audit.exitCode !== 0 || !auditOutput.includes(AUDIT_CLEAN)) {
    return refuse([`dafny audit did not report 0 findings`, auditOutput.trim()]);
  }
  const unverified = unverifiedTheorems(verify.stdout, input.theorems);
  if (unverified.length > 0) {
    return refuse(
      unverified.map(
        (t) => `theorem not verified in ${input.file} or its includes: ${t}; name it as Dafny's verification log does, qualified by every enclosing module and type`
      )
    );
  }

  const version = await runDafny(root, ["--version"], run);
  const dafnyVersion = version.stdout.trim();
  if (version.exitCode !== 0 || !/^\d+\.\d+\.\d+\S*$/.test(dafnyVersion)) {
    return refuse([`could not read the Dafny version: ${dafnyVersion}`]);
  }

  const headAfter = await git(root, ["rev-parse", "HEAD"]);
  if (!headAfter.ok || headAfter.stdout.trim() !== commit) {
    return refuse([`HEAD moved from ${commit} while Dafny ran: ${headAfter.stdout.trim()}`]);
  }
  const dirtyAfter = await treeChanges(root);
  if (dirtyAfter === null) return refuse([`git could not read the work tree state in ${root}`]);
  if (dirtyAfter.length > 0) return refuse(dirtyAfter.map((l) => `work tree changed while Dafny ran: ${l}`));

  const record = buildRecord({ ...input, includes: included.files.slice(1), commit, dafnyVersion, image, imageId });
  if (out === null || input.outputPath === undefined) return { success: true, errors: [], record, writtenTo: null };
  const stillSafe = await outputTarget(root, input.outputPath);
  if (stillSafe.error !== null) return refuse([stillSafe.error], record);
  const temp = `${out}.${process.pid}.tmp`;
  try {
    await writeFile(temp, JSON.stringify(record, null, 2) + "\n", { encoding: "utf-8", flag: "wx" });
  } catch (err) {
    return refuse([`could not write ${out}: ${(err as Error).message}`], record);
  }
  try {
    await rename(temp, out);
  } catch (err) {
    // The temp file is ours here, because "wx" created it, and a leftover would dirty the tree.
    await unlink(temp).catch(() => undefined);
    return refuse([`could not write ${out}: ${(err as Error).message}`], record);
  }
  return { success: true, errors: [], record, writtenTo: out };
}
