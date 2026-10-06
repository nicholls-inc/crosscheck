import { execFile } from "node:child_process";
import { lstat, readFile, writeFile } from "node:fs/promises";
import { isAbsolute, resolve as resolvePath } from "node:path";
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
    !input.file.endsWith(".dfy")
  ) {
    errors.push(`file must be a relative path to a .dfy file with no "." or ".." segment: ${input.file}`);
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

function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function declares(source: string, keywords: string, name: string): boolean {
  const pattern = new RegExp(
    `\\b(?:${keywords})\\s+(?:\\{:[^}]*\\}\\s*)*${escapeRegExp(name)}(?![A-Za-z0-9_'?])`
  );
  return pattern.test(source);
}

export function undeclaredTheorems(source: string, theorems: string[]): string[] {
  return theorems.filter((theorem) => {
    const parts = theorem.split(".");
    const last = parts.pop()!;
    return (
      !declares(source, "lemma|method|function|predicate", last) ||
      parts.some((q) => !declares(source, "module", q))
    );
  });
}

export function shellQuote(s: string): string {
  return `'${s.replace(/'/g, `'\\''`)}'`;
}

export function rerunCommand(image: string, file: string): string {
  const run = `docker run --rm --network=none -v "$PWD":/work ${shellQuote(image)}`;
  const path = shellQuote(`/work/${file}`);
  return `${run} verify ${path} && ${run} audit ${path} 2>&1 | grep -q '${AUDIT_CLEAN}'`;
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
        rerun: { command: rerunCommand(facts.image, facts.file), exit_code: 0 },
      },
    ],
  };
}

function git(cwd: string, args: string[]): Promise<{ ok: boolean; stdout: string }> {
  return new Promise((done) => {
    execFile("git", ["--literal-pathspecs", "-C", cwd, ...args], (err, stdout) => {
      done({ ok: err === null, stdout: String(stdout) });
    });
  });
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

  const status = await git(root, ["status", "--porcelain", "--untracked-files=all"]);
  const dirty = status.stdout.split("\n").filter((l) => l !== "");
  if (!status.ok) return refuse([`git status failed in ${root}`]);
  if (dirty.length > 0) return refuse(dirty.map((l) => `work tree differs from ${commit}: ${l}`));
  if (!(await git(root, ["ls-files", "--error-unmatch", "--", input.file])).ok) {
    return refuse([`not committed: ${input.file}`]);
  }

  let source: string;
  try {
    if ((await lstat(resolvePath(root, input.file))).isSymbolicLink()) {
      return refuse([`${input.file} is a symbolic link; pass the file it points to`]);
    }
    source = await readFile(resolvePath(root, input.file), "utf-8");
  } catch (err) {
    return refuse([`could not read ${input.file}: ${(err as Error).message}`]);
  }
  const undeclared = undeclaredTheorems(source, input.theorems);
  if (undeclared.length > 0) {
    return refuse(undeclared.map((t) => `theorem not declared in ${input.file}: ${t}`));
  }

  const path = `/work/${input.file}`;
  const verify = await runDafny(root, ["verify", path]);
  if (verify.timedOut || verify.exitCode !== 0) {
    return refuse([
      `dafny verify exited ${verify.timedOut ? "on timeout" : verify.exitCode}`,
      (verify.stdout + "\n" + verify.stderr).trim(),
    ]);
  }
  const audit = await runDafny(root, ["audit", path]);
  const auditOutput = audit.stdout + "\n" + audit.stderr;
  if (audit.timedOut || audit.exitCode !== 0 || !auditOutput.includes(AUDIT_CLEAN)) {
    return refuse([`dafny audit did not report 0 findings`, auditOutput.trim()]);
  }

  const version = await runDafny(root, ["--version"]);
  const dafnyVersion = version.stdout.trim();
  if (version.exitCode !== 0 || !/^\d+\.\d+\.\d+\S*$/.test(dafnyVersion)) {
    return refuse([`could not read the Dafny version: ${dafnyVersion}`]);
  }
  const image = getDockerImage();
  const imageId = await dockerImageId(image);
  if (imageId === null) return refuse([`could not read the ID of image ${image}`]);

  const record = buildRecord({ ...input, commit, dafnyVersion, image, imageId });
  if (input.outputPath === undefined) {
    return { success: true, errors: [], record, writtenTo: null };
  }
  const out = resolvePath(root, input.outputPath);
  try {
    await writeFile(out, JSON.stringify(record, null, 2) + "\n", "utf-8");
  } catch (err) {
    return refuse([`could not write ${out}: ${(err as Error).message}`], record);
  }
  return { success: true, errors: [], record, writtenTo: out };
}
