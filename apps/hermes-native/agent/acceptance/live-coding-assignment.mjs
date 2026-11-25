import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { mkdir, readFile, realpath, rename, writeFile } from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";

function pendingSubmissionCount(state) {
  const count = state?.chat?.submission_pending_count;
  if (Number.isSafeInteger(count) && count >= 0) return count;
  if (state?.chat?.submission_pending === true) return 1;
  return null;
}

function sendingStatus(state) {
  return /^(?:sending|stopping|working|running|processing)\b/i.test(String(state?.status ?? "").trim());
}

export function hasPendingWork(state) {
  const chat = state?.chat ?? {};
  const count = pendingSubmissionCount(state);
  return count === null
    || count > 0
    || chat.submission_pending !== false
    || chat.interrupt_pending === true
    || sendingStatus(state);
}

export function classifyTurnCompletion({ state, priorAssistantCount, liveAssistantCount, explicitError = false }) {
  const chat = state?.chat ?? {};
  if (hasPendingWork(state)) return { kind: "wait", reason: "pending_work" };
  if (chat.submission_uncertain === true) return { kind: "unknown", reason: "submission_uncertain" };
  if (chat.approval_pending_count > 0) return { kind: "approval_required" };
  if (chat.clarify_pending_count > 0) return { kind: "clarification_required" };
  if (explicitError) return { kind: "error" };
  if (liveAssistantCount > priorAssistantCount) return { kind: "settled" };
  return { kind: "wait", reason: "assistant_terminal_not_observed" };
}

function selfCheck() {
  const base = {
    status: "Ready",
    chat: {
      submission_pending: false,
      submission_pending_count: 0,
      submission_uncertain: false,
      interrupt_pending: false,
      approval_pending_count: 0,
      clarify_pending_count: 0,
    },
  };
  const priorAssistantCount = 2;
  const checks = [
    ["user row growth alone", { state: base, priorAssistantCount, liveAssistantCount: 2 }, "wait"],
    ["assistant row while sending", {
      state: { ...base, status: "Sending message to Hermes…" }, priorAssistantCount, liveAssistantCount: 3,
    }, "wait"],
    ["assistant row with pending submission", {
      state: { ...base, chat: { ...base.chat, submission_pending: true, submission_pending_count: 1 } },
      priorAssistantCount, liveAssistantCount: 3,
    }, "wait"],
    ["approval while submission is pending", {
      state: { ...base, chat: { ...base.chat, submission_pending: true, submission_pending_count: 1, approval_pending_count: 1 } },
      priorAssistantCount, liveAssistantCount: 2,
    }, "wait"],
    ["new terminal assistant row", { state: base, priorAssistantCount, liveAssistantCount: 3 }, "settled"],
    ["explicit approval with no pending work", {
      state: { ...base, chat: { ...base.chat, approval_pending_count: 1 } }, priorAssistantCount, liveAssistantCount: 2,
    }, "approval_required"],
    ["explicit error with no pending work", {
      state: base, priorAssistantCount, liveAssistantCount: 2, explicitError: true,
    }, "error"],
  ];
  for (const [label, input, expected] of checks) {
    const actual = classifyTurnCompletion(input).kind;
    if (actual !== expected) throw new Error(`${label}: expected ${expected}, got ${actual}`);
  }
  process.stdout.write(`live coding acceptance completion-gate self-check passed (${checks.length} cases)\n`);
}

if (process.argv.includes("--self-check")) {
  selfCheck();
  process.exit(0);
}

const required = ["HERMES_TEST_PROFILE", "HERMES_TEST_PROJECT", "HERMES_TEST_EXE", "HERMES_TEST_SCRIPTS"];
for (const name of required) {
  if (!process.env[name]) throw new Error(`${name} must name the isolated test environment`);
}

const profile = await realpath(process.env.HERMES_TEST_PROFILE);
const project = await realpath(process.env.HERMES_TEST_PROJECT);
const executable = await realpath(process.env.HERMES_TEST_EXE);
const temp = await realpath(process.env.TEMP ?? process.env.TMP ?? "");
const profileRoot = path.dirname(profile);
if (path.dirname(profileRoot).toLowerCase() !== temp.toLowerCase()
  || !/^HermesNative-Delivery-Test-[0-9-]+$/i.test(path.basename(profileRoot))
  || path.basename(profile).toLowerCase() !== "gateway-profile"
  || project.toLowerCase() !== path.join(profileRoot, "project").toLowerCase()) {
  throw new Error("Refusing live acceptance outside the isolated HermesNative-Delivery-Test temp profile/project pair");
}

const expectedFiles = ["calc.py", "test_calc.py"];
const before = Object.fromEntries(await Promise.all(expectedFiles.map(async (name) => {
  const bytes = await readFile(path.join(project, name));
  return [name, createHash("sha256").update(bytes).digest("hex")];
})));
const { HermesPage } = await import(pathToFileURL(path.resolve("apps/hermes-native/agent/hermes-page.mjs")));
const pinnedHermesScripts = await realpath(process.env.HERMES_TEST_SCRIPTS);
if (path.dirname(path.dirname(pinnedHermesScripts)).toLowerCase() !== temp.toLowerCase()
  || path.basename(path.dirname(pinnedHermesScripts)).toLowerCase() !== "hermes-native-pinned-test-venv") {
  throw new Error("HERMES_TEST_SCRIPTS must be the pinned Hermes test virtual environment under the system temp directory");
}
const env = {
  ...process.env,
  HERMES_HOME: profile,
  PATH: `${pinnedHermesScripts};${process.env.PATH ?? ""}`,
};
const page = await HermesPage.launch({
  command: executable,
  args: ["--stdio-live-gateway"],
  cwd: project,
  env,
  readyTimeoutMs: 60_000,
  requestTimeoutMs: 15_000,
});

const collect = (node, out = []) => {
  if (node?.name || node?.value) out.push({ role: node.role, name: node.name, value: node.value });
  for (const child of node?.children ?? []) collect(child, out);
  return out;
};
const hashFile = async (name) => createHash("sha256").update(await readFile(path.join(project, name))).digest("hex");
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

let outcome = "unknown";
let finalState;
let controls = [];
let promptSent = false;
const startedAt = new Date().toISOString();
const stamp = new Date().toISOString().replaceAll(":", "-").replaceAll(".", "-");
const artifactDir = path.resolve("artifacts/agent-ui");
await mkdir(artifactDir, { recursive: true });

try {
  await page.getByRole("Button", { name: "Settings" }).click();
  await sleep(2_500);
  await page.getByRole("Button", { name: "Close" }).click();
  let attached;
  const attachDeadline = Date.now() + 60_000;
  do {
    attached = await page.state();
    if (attached.gateway?.state === "connected") break;
    await sleep(1_000);
  } while (Date.now() < attachDeadline);
  if (attached.gateway?.state !== "connected") {
    throw new Error(`Isolated Gateway connection failed: ${attached.gateway?.state ?? "missing state"}`);
  }
  const beforeScreenshot = `live-coding-before-${stamp}.png`;
  const afterScreenshot = `live-coding-after-${stamp}.png`;
  await page.screenshot(beforeScreenshot);

  const prompt = [
    "In this isolated coding test project only, inspect calc.py and test_calc.py.",
    "Fix add(a, b) so it performs ordinary integer addition for positive and negative values.",
    "Keep the existing regression test. Use Hermes tools to inspect and edit these project files.",
    "Use the terminal only to run `python -m unittest -q` from this project directory.",
    "Do not read or change files outside this project and do not use network access.",
    "Then summarize files changed and the test result.",
  ].join(" ");
  await page.getByRole("MultilineTextInput", { name: "Message composer" }).fill(prompt);
  const beforeSubmit = await page.state();
  const priorAssistantCount = collect(await page.tree())
    .filter((item) => item.role === "Group" && /^assistant\s*:/i.test(item.name ?? "")).length;
  promptSent = true;
  await page.getByRole("Button", { name: "Send" }).click();

  const deadline = Date.now() + 5 * 60_000;
  let warnedPendingTimeout = false;
  while (true) {
    finalState = await page.state();
    const liveControls = collect(await page.tree());
    const liveAssistantCount = liveControls
      .filter((item) => item.role === "Group" && /^assistant\s*:/i.test(item.name ?? "")).length;
    const liveError = liveControls.some((item) => /billing or credits exhausted|HTTP 402|insufficient balance/i.test(item.value ?? item.name ?? ""));
    const completion = classifyTurnCompletion({
      state: finalState,
      priorAssistantCount,
      liveAssistantCount,
      explicitError: liveError,
    });
    if (completion.kind !== "wait") {
      outcome = ({
        settled: "turn_settled",
        approval_required: "approval_or_clarification_required",
        clarification_required: "approval_or_clarification_required",
        error: "agent_turn_failed",
        unknown: "submission_outcome_unknown",
      })[completion.kind];
      break;
    }
    if (Date.now() >= deadline && !warnedPendingTimeout) {
      warnedPendingTimeout = true;
      process.stderr.write("No terminal assistant/approval/error state after five minutes; keeping Hermes Native attached until pending work settles.\n");
    }
    await sleep(1_000);
  }

  const tree = await page.tree();
  controls = collect(tree);
  const assistantError = controls
    .map((item) => item.value ?? item.name ?? "")
    .find((value) => /assistant.*error|billing or credits exhausted|payment.*failed|HTTP 402|insufficient balance/i.test(value)) ?? null;
  if (outcome === "turn_settled" && assistantError) {
    outcome = /HTTP 402|insufficient balance|billing or credits exhausted/i.test(assistantError)
      ? "provider_blocked"
      : "agent_turn_failed";
  }
  const after = Object.fromEntries(await Promise.all(expectedFiles.map(async (name) => {
    try { return [name, await hashFile(name)]; }
    catch { return [name, null]; }
  })));
  const changedFiles = expectedFiles.filter((name) => after[name] && after[name] !== before[name]);
  let unitTest = { status: "not_run", exit_code: null, output: "" };
  if (outcome === "turn_settled" && changedFiles.includes("calc.py")) {
    const test = spawnSync("python", ["-m", "unittest", "-q"], {
      cwd: project,
      encoding: "utf8",
      timeout: 30_000,
      env: { ...process.env, PATH: `${pinnedHermesScripts};${process.env.PATH ?? ""}` },
      windowsHide: true,
    });
    unitTest = {
      status: test.error ? "error" : test.status === 0 ? "passed" : "failed",
      exit_code: test.status,
      output: `${test.stdout ?? ""}${test.stderr ?? ""}`.slice(0, 4_000),
    };
  }
  if (outcome === "turn_settled" && changedFiles.includes("calc.py") && unitTest.status === "passed") {
    outcome = "coding_assignment_passed";
  }

  await page.screenshot(afterScreenshot);
  const summary = {
    schema_version: 1,
    started_at: startedAt,
    finished_at: new Date().toISOString(),
    outcome,
    failure_summary: assistantError?.slice(0, 500) ?? null,
    gateway_state: finalState?.gateway?.state ?? "connected",
    active_runtime_session_id: finalState?.chat?.active_runtime_session_id ?? null,
    transcript_message_count: finalState?.transcript_message_count ?? null,
    app_status: finalState?.status ?? null,
    submission_pending: finalState?.chat?.submission_pending ?? null,
    submission_pending_count: finalState?.chat?.submission_pending_count ?? null,
    submission_uncertain: finalState?.chat?.submission_uncertain ?? null,
    approval_pending_count: finalState?.chat?.approval_pending_count ?? 0,
    clarify_pending_count: finalState?.chat?.clarify_pending_count ?? 0,
    changed_files: changedFiles,
    unit_test: unitTest,
    visible_controls: controls.filter((item) => /approve|allow|deny|clarify|stop|tool|running|failed|retry|test|terminal|error|credit|payment/i.test(`${item.name ?? ""} ${item.value ?? ""}`)).slice(-60),
    screenshot_files: [
      `apps/hermes-native/artifacts/agent-ui/${beforeScreenshot}`,
      `apps/hermes-native/artifacts/agent-ui/${afterScreenshot}`,
    ],
  };
  const resultPath = path.join(artifactDir, `live-coding-result-${stamp}.json`);
  const tempPath = `${resultPath}.tmp`;
  await writeFile(tempPath, `${JSON.stringify(summary, null, 2)}\n`, { flag: "wx" });
  await rename(tempPath, resultPath);
  process.stdout.write(`${JSON.stringify({ ...summary, result_file: resultPath })}\n`);
  if (outcome !== "coding_assignment_passed") process.exitCode = 2;
} finally {
  if (promptSent && !page.closed) {
    let safeState = finalState;
    let warnedUnsafeClose = false;
    while (hasPendingWork(safeState)) {
      if (!warnedUnsafeClose) {
        warnedUnsafeClose = true;
        process.stderr.write("Keeping the headless Hermes session open while work is sending or pending.\n");
      }
      await sleep(1_000);
      try {
        safeState = await page.state();
      } catch {
        safeState = null;
      }
      if (page.closed) break;
    }
  }
  try {
    await page.close();
  } catch (error) {
    process.stderr.write(`Headless app shutdown reported: ${error?.message ?? String(error)}\n`);
    if (process.exitCode === undefined || process.exitCode === 0) process.exitCode = 3;
  }
}
