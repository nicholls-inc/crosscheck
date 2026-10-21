import { execFile } from "node:child_process";

// `git ls-files -v` prints a line per tracked file, so the 1 MiB default would refuse a large repository.
export const GIT_MAX_BUFFER = 512 * 1024 * 1024;

export interface GitOptions {
  input?: string;
  env?: NodeJS.ProcessEnv;
}

export function gitBytes(cwd: string, args: string[], opts: GitOptions = {}): Promise<{ ok: boolean; stdout: Buffer }> {
  return new Promise((done) => {
    const child = execFile(
      "git",
      ["--literal-pathspecs", "-C", cwd, ...args],
      { maxBuffer: GIT_MAX_BUFFER, encoding: "buffer", env: opts.env },
      (err, stdout) => done({ ok: err === null, stdout })
    );
    child.stdin?.on("error", () => undefined);
    child.stdin?.end(opts.input ?? "");
  });
}

export async function git(cwd: string, args: string[], opts: GitOptions = {}): Promise<{ ok: boolean; stdout: string }> {
  const { ok, stdout } = await gitBytes(cwd, args, opts);
  return { ok, stdout: stdout.toString() };
}

export async function treeChanges(root: string): Promise<string[] | null> {
  const status = await git(root, ["status", "--porcelain", "--untracked-files=all"]);
  const tags = await git(root, ["ls-files", "-v"]);
  if (!status.ok || !tags.ok) return null;
  const hidden = tags.stdout
    .split("\n")
    .filter((l) => /^[a-zS] /.test(l))
    .map((l) => `${/^[sS]/.test(l) ? "skip-worktree" : "assume-unchanged"} hides changes to ${l.slice(2)}`);
  return [...status.stdout.split("\n").filter((l) => l !== ""), ...hidden];
}
