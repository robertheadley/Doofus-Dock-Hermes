import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { createInterface } from "node:readline";
import { fileURLToPath } from "node:url";

const workspaceRoot = fileURLToPath(new URL("../../../", import.meta.url));
const defaultArgs = [
  "run",
  "-p",
  "hermes-native",
  "--features",
  "agent-automation",
  "--bin",
  "hermes-automation",
  "--",
  "--stdio",
];

export class HermesAgentError extends Error {
  constructor(code, message, requestId) {
    super(message);
    this.name = "HermesAgentError";
    this.code = code;
    this.requestId = requestId;
  }
}

/** A Playwright-shaped client for the Hermes Native semantic UI child process. */
export class HermesPage {
  static async launch({
    command = "cargo",
    args = defaultArgs,
    cwd = workspaceRoot,
    env = process.env,
    readyTimeoutMs = 30_000,
    requestTimeoutMs = 10_000,
  } = {}) {
    const child = spawn(command, args, {
      cwd,
      env,
      stdio: ["pipe", "pipe", "pipe"],
      windowsHide: true,
    });
    const page = new HermesPage(child, requestTimeoutMs);
    await page._waitUntilReady(readyTimeoutMs);
    return page;
  }

  constructor(child, requestTimeoutMs) {
    this.child = child;
    this.requestTimeoutMs = requestTimeoutMs;
    this.generation = 0;
    this.pending = new Map();
    this.stderr = "";
    this.closed = false;
    this.actionTail = Promise.resolve();
    this.ready = new Promise((resolve, reject) => {
      this.resolveReady = resolve;
      this.rejectReady = reject;
    });
    this.lines = createInterface({ input: child.stdout });
    this.lines.on("line", (line) => this._onLine(line));
    child.stderr.setEncoding("utf8");
    child.stderr.on("data", (chunk) => {
      this.stderr = (this.stderr + chunk).slice(-8192);
    });
    child.on("error", (error) => this._failAll("process_error", error.message));
    child.on("exit", (code, signal) => {
      this.closed = true;
      const detail = `child exited (code=${code}, signal=${signal})`;
      this._failAll("process_exited", `${detail}${this.stderr ? `; stderr: ${this.stderr.trim()}` : ""}`);
    });
  }

  async _waitUntilReady(timeoutMs) {
    let timer;
    try {
      await Promise.race([
        this.ready,
        new Promise((_, reject) => {
          timer = setTimeout(() => reject(new HermesAgentError("ready_timeout", "Hermes agent did not become ready")), timeoutMs);
        }),
      ]);
    } catch (error) {
      this.child.kill();
      throw error;
    } finally {
      clearTimeout(timer);
    }
  }

  _onLine(line) {
    let message;
    try {
      message = JSON.parse(line);
    } catch {
      this._failAll("invalid_response", `Hermes agent emitted invalid JSON: ${line.slice(0, 200)}`);
      return;
    }
    if (message.type === "ready") {
      this.generation = message.state_generation ?? 0;
      this.resolveReady(message);
      return;
    }
    const pending = this.pending.get(message.id);
    if (!pending) return;
    clearTimeout(pending.timer);
    this.pending.delete(message.id);
    if (message.status !== "succeeded") {
      pending.reject(new HermesAgentError(
        message.error?.code ?? "request_failed",
        message.error?.message ?? "Hermes UI request failed",
        message.id,
      ));
      return;
    }
    const generation = message.result?.state_generation_after ?? message.result?.state_generation;
    if (Number.isInteger(generation)) this.generation = generation;
    pending.resolve(message.result);
  }

  _failAll(code, message) {
    if (this.rejectReady) {
      this.rejectReady(new HermesAgentError(code, message));
      this.rejectReady = null;
    }
    for (const [id, pending] of this.pending) {
      clearTimeout(pending.timer);
      pending.reject(new HermesAgentError(code, message, id));
    }
    this.pending.clear();
  }

  async request(method, params = {}, { timeoutMs = this.requestTimeoutMs, mutation = false } = {}) {
    await this.ready;
    if (this.closed) throw new HermesAgentError("process_exited", "Hermes agent process has exited");
    const id = randomUUID();
    const payload = `${JSON.stringify({ version: 1, id, method, params })}\n`;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        reject(new HermesAgentError(
          mutation ? "request_timeout_outcome_unknown" : "request_timeout",
          mutation
            ? `Mutation ${id} timed out; its outcome is unknown and was not retried`
            : `Request ${id} timed out`,
          id,
        ));
      }, timeoutMs);
      this.pending.set(id, { resolve, reject, timer });
      this.child.stdin.write(payload, (error) => {
        if (!error) return;
        clearTimeout(timer);
        this.pending.delete(id);
        reject(new HermesAgentError("pipe_write_failed", error.message, id));
      });
    });
  }

  async _mutate(method, params) {
    const operation = this.actionTail.then(async () => {
      params.expected_state_generation = this.generation;
      return this.request(method, params, { mutation: true });
    });
    this.actionTail = operation.catch(() => {});
    return operation;
  }

  state() { return this.request("state.get"); }
  tree() { return this.request("ui.tree"); }
  diagnostics() { return this.request("diagnostics.bundle"); }
  events() { return this.request("events.list"); }
  capabilities() { return this.request("capabilities.list"); }

  getByRole(role, { name } = {}) {
    if (typeof role !== "string" || !role || typeof name !== "string" || !name) {
      throw new TypeError("getByRole requires a role and exact accessible name");
    }
    return new HermesLocator(this, { role, name });
  }

  getByTestId(testId) {
    if (typeof testId !== "string" || !testId) throw new TypeError("test ID is required");
    return new HermesLocator(this, { test_id: testId });
  }

  async press(key, modifiers = []) {
    return this._mutate("actions.keypress", { key, modifiers });
  }

  async screenshot(filename) {
    return this.request("screenshots.capture", { filename }, { mutation: true });
  }

  async close() {
    if (this.closed) return;
    try {
      await this.request("protocol.close", {}, { timeoutMs: 2_000, mutation: true });
    } finally {
      this.child.stdin.end();
      await Promise.race([
        new Promise((resolve) => this.child.once("exit", resolve)),
        new Promise((resolve) => setTimeout(resolve, 2_000)),
      ]);
      if (!this.closed) this.child.kill();
      this.lines.close();
    }
  }
}

export class HermesLocator {
  constructor(page, query) {
    this.page = page;
    this.query = query;
  }

  async click() { return this.page._mutate("actions.click", { ...this.query }); }
  async rightClick() { return this.page._mutate("actions.right_click", { ...this.query }); }
  async fill(text) { return this.page._mutate("actions.fill", { ...this.query, text }); }
  async type(text) { return this.page._mutate("actions.type", { ...this.query, text }); }
  async selectOption(option) { return this.page._mutate("actions.select", { ...this.query, option }); }
  async scroll({ deltaX = 0, deltaY }) {
    return this.page._mutate("actions.scroll", { ...this.query, delta_x: deltaX, delta_y: deltaY });
  }

  async dragTo(target, { steps } = {}) {
    const params = { ...this.query };
    if (target instanceof HermesLocator) {
      if (target.page !== this.page) throw new TypeError("drag locators must belong to the same Hermes page");
      if (target.query.test_id) params.to_test_id = target.query.test_id;
      else Object.assign(params, { to_role: target.query.role, to_name: target.query.name });
    } else if (Number.isFinite(target?.x) && Number.isFinite(target?.y)) {
      params.to_x = target.x;
      params.to_y = target.y;
    } else {
      throw new TypeError("drag target must be a Hermes locator or {x, y}");
    }
    if (steps !== undefined) params.steps = steps;
    return this.page._mutate("actions.drag", params);
  }

  async waitFor({ timeoutMs = 2_000 } = {}) {
    return this.page.request("ui.wait_for", { ...this.query, timeout_ms: timeoutMs }, { timeoutMs: timeoutMs + 1_000 });
  }

  async _matches() {
    const root = await this.page.tree();
    const matches = [];
    const visit = (node) => {
      if (this.query.test_id
        ? node.test_id === this.query.test_id
        : node.role === this.query.role && node.name === this.query.name) matches.push(node);
      for (const child of node.children ?? []) visit(child);
    };
    visit(root);
    return matches;
  }

  async count() { return (await this._matches()).length; }
  async isVisible() { const nodes = await this._matches(); return nodes.length === 1 && nodes[0].visible === true; }
  async isEnabled() { const nodes = await this._matches(); return nodes.length === 1 && nodes[0].enabled === true; }
  async boundingBox() {
    const nodes = await this._matches();
    if (nodes.length !== 1 || !Array.isArray(nodes[0].bounds)) return null;
    const [x, y, width, height] = nodes[0].bounds;
    return { x, y, width, height };
  }
}
