import { describe, it, expect, vi, beforeEach } from "vitest";
import { EventEmitter } from "node:events";
import { Readable } from "node:stream";
import { spawn } from "node:child_process";

vi.mock("node:child_process");

import { dockerImageId, runDafny } from "../../docker.js";

function createMockProcess() {
  const proc = new EventEmitter() as any;
  proc.stdout = new Readable({ read() {} });
  proc.stderr = new Readable({ read() {} });
  proc.kill = vi.fn();
  return proc;
}

describe("runDafny", () => {
  beforeEach(() => {
    delete process.env.DAFNY_DOCKER_IMAGE;
  });

  it("constructs correct docker args", async () => {
    const mockProc = createMockProcess();
    vi.mocked(spawn).mockReturnValue(mockProc as any);

    const promise = runDafny("/tmp/workdir", ["verify", "/work/program.dfy"]);

    // Emit close immediately
    mockProc.emit("close", 0);

    await promise;

    expect(spawn).toHaveBeenCalledWith("docker", [
      "run",
      "--rm",
      "--network=none",
      "--memory=512m",
      "--cpus=1",
      "-v",
      "/tmp/workdir:/work",
      "crosscheck-dafny:latest",
      "verify",
      "/work/program.dfy",
    ]);
  });

  it("runs a named image with the work tree mounted read-only when asked", async () => {
    const mockProc = createMockProcess();
    vi.mocked(spawn).mockReturnValue(mockProc as any);

    const promise = runDafny("/tmp/workdir", ["audit", "/work/a.dfy"], { image: "sha256:feed", readOnly: true });
    mockProc.emit("close", 0);
    await promise;

    expect(vi.mocked(spawn).mock.lastCall).toEqual([
      "docker",
      [
        "run",
        "--rm",
        "--network=none",
        "--memory=512m",
        "--cpus=1",
        "-v",
        "/tmp/workdir:/work:ro",
        "sha256:feed",
        "audit",
        "/work/a.dfy",
      ],
    ]);
  });

  it.each([
    [{ image: "sha256:feed" }, "sha256:feed", "/tmp/workdir:/work"],
    [{ readOnly: true }, "crosscheck-dafny:latest", "/tmp/workdir:/work:ro"],
  ])("mounts read-only only when asked (%j)", async (options, image, mount) => {
    const mockProc = createMockProcess();
    vi.mocked(spawn).mockReturnValue(mockProc as any);

    const promise = runDafny("/tmp/workdir", ["verify"], options);
    mockProc.emit("close", 0);
    await promise;

    const args = vi.mocked(spawn).mock.lastCall![1] as string[];
    expect(args.slice(6)).toEqual([mount, image, "verify"]);
  });

  it("collects stdout and stderr", async () => {
    const mockProc = createMockProcess();
    vi.mocked(spawn).mockReturnValue(mockProc as any);

    const promise = runDafny("/tmp/workdir", ["verify", "/work/program.dfy"]);

    mockProc.stdout.emit("data", Buffer.from("verification output"));
    mockProc.stderr.emit("data", Buffer.from("some warning"));
    mockProc.emit("close", 0);

    const result = await promise;

    expect(result.stdout).toBe("verification output");
    expect(result.stderr).toBe("some warning");
  });

  it("resolves with exit code, stdout, stderr, and timedOut false on close", async () => {
    const mockProc = createMockProcess();
    vi.mocked(spawn).mockReturnValue(mockProc as any);

    const promise = runDafny("/tmp/workdir", ["verify", "/work/program.dfy"]);

    mockProc.stdout.emit("data", Buffer.from("ok"));
    mockProc.emit("close", 0);

    const result = await promise;

    expect(result).toEqual({
      exitCode: 0,
      stdout: "ok",
      stderr: "",
      timedOut: false,
    });
  });

  it("defaults null exit code to 1", async () => {
    const mockProc = createMockProcess();
    vi.mocked(spawn).mockReturnValue(mockProc as any);

    const promise = runDafny("/tmp/workdir", ["verify", "/work/program.dfy"]);
    mockProc.emit("close", null);

    const result = await promise;

    expect(result.exitCode).toBe(1);
  });

  it("sends SIGKILL and sets timedOut on timeout", async () => {
    vi.useFakeTimers();

    const mockProc = createMockProcess();
    vi.mocked(spawn).mockReturnValue(mockProc as any);

    const promise = runDafny("/tmp/workdir", ["verify", "/work/program.dfy"]);

    // Advance past the 120s timeout
    vi.advanceTimersByTime(120_001);

    expect(mockProc.kill).toHaveBeenCalledWith("SIGKILL");

    // Process closes after being killed
    mockProc.emit("close", null);

    const result = await promise;

    expect(result.timedOut).toBe(true);
    expect(result.exitCode).toBe(1);

    vi.useRealTimers();
  });

});

describe("dockerImageId", () => {
  function start() {
    const mockProc = createMockProcess();
    vi.mocked(spawn).mockReturnValue(mockProc as any);
    return { mockProc, result: dockerImageId("img:tag") };
  }

  it("inspects the image and returns the trimmed ID", async () => {
    const { mockProc, result } = start();
    mockProc.stdout.emit("data", Buffer.from("sha256:abc\n"));
    mockProc.emit("close", 0);
    expect(await result).toBe("sha256:abc");
    expect(vi.mocked(spawn).mock.lastCall![0]).toBe("docker");
    expect(vi.mocked(spawn).mock.lastCall![1]).toEqual(["image", "inspect", "--format", "{{.Id}}", "img:tag"]);
  });

  it("returns null when docker exits non-zero", async () => {
    const { mockProc, result } = start();
    mockProc.stdout.emit("data", Buffer.from("sha256:abc\n"));
    mockProc.emit("close", 1);
    expect(await result).toBeNull();
  });

  it("returns null when the output is empty", async () => {
    const { mockProc, result } = start();
    mockProc.stdout.emit("data", Buffer.from("\n"));
    mockProc.emit("close", 0);
    expect(await result).toBeNull();
  });

  it("returns null when docker cannot start", async () => {
    const { mockProc, result } = start();
    mockProc.stdout.emit("data", Buffer.from("sha256:abc\n"));
    mockProc.emit("error", new Error("ENOENT"));
    expect(await result).toBeNull();
  });
});
