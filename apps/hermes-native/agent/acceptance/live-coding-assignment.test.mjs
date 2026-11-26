import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

const script = fileURLToPath(new URL("./live-coding-assignment.mjs", import.meta.url));
const repository = path.resolve(path.dirname(script), "../../../..");

test("live coding completion gate ignores user-row growth and waits for idle assistant outcome", () => {
  const result = spawnSync(process.execPath, [script, "--self-check"], {
    cwd: repository,
    encoding: "utf8",
    timeout: 10_000,
    windowsHide: true,
  });
  assert.equal(result.error, undefined, result.error?.message);
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /self-check passed \(9 cases\)/);
  assert.doesNotMatch(result.stdout + result.stderr, /Gateway|HERMES_TEST_PROFILE/);
});
