import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { createInterface } from "node:readline";
import { join } from "node:path";
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
  constructor(code, message, requestId, details) {
    super(message);
    this.name = "HermesAgentError";
    this.code = code;
    this.requestId = requestId;
    this.details = details;
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

  static async launchNative({
    command = join(workspaceRoot, "target", "debug", "hermes-automation.exe"),
    args = ["--native-window", "--stdio"],
    cwd = workspaceRoot,
    env = process.env,
    readyTimeoutMs = 30_000,
    requestTimeoutMs = 10_000,
  } = {}) {
    if (process.platform !== "win32") {
      throw new HermesAgentError("unsupported_platform", "The native Hermes window protocol is available only on Windows");
    }
    return HermesPage.launch({ command, args, cwd, env, readyTimeoutMs, requestTimeoutMs });
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
      this.protocol = message.protocol ?? "hermes-native-agent-ui/1";
      this.generation = message.state_generation ?? 0;
      this.resolveReady(message);
      return;
    }
    const pending = this.pending.get(message.id);
    if (!pending) return;
    clearTimeout(pending.timer);
    this.pending.delete(message.id);
    if (message.status !== "succeeded") {
      if (Number.isInteger(message.error?.state_generation)) {
        this.generation = message.error.state_generation;
      }
      pending.reject(new HermesAgentError(
        message.error?.code ?? "request_failed",
        message.error?.message ?? "Hermes UI request failed",
        message.id,
        message.error,
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
  async tree() {
    const result = await this.request("ui.tree");
    if (this.protocol !== "hermes-native-window/1" || !Array.isArray(result.tree?.nodes)) return result;
    const source = result.tree;
    const rootIndex = source.root;
    const rootBounds = source.nodes[rootIndex]?.bounds ?? { x: 0, y: 0, width: 0, height: 0 };
    const nodes = source.nodes.map((node) => {
      const bounds = node.bounds;
      const localBounds = bounds && node.bounds_valid !== false && {
        x: bounds.x - rootBounds.x,
        y: bounds.y - rootBounds.y,
        width: bounds.width,
        height: bounds.height,
      };
      const visible = Boolean(localBounds
        && localBounds.width > 0
        && localBounds.height > 0
        && localBounds.x < rootBounds.width
        && localBounds.y < rootBounds.height
        && localBounds.x + localBounds.width > 0
        && localBounds.y + localBounds.height > 0);
      return {
        ...node,
        test_id: node.automation_id || undefined,
        visible,
        enabled: node.enabled === true,
        focused: node.focused === true,
        bounds: localBounds ? [localBounds.x, localBounds.y, localBounds.width, localBounds.height] : null,
        children: [],
      };
    });
    for (const node of source.nodes) {
      if (Number.isInteger(node.parent_id) && nodes[node.parent_id]) nodes[node.parent_id].children.push(nodes[node.id]);
    }
    return nodes[rootIndex] ?? { role: "Window", name: "Hermes", children: [] };
  }
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
    if (this.protocol === "hermes-native-window/1") {
      try {
        return await this._mutate("ui.keypress", { key, modifiers });
      } catch (error) {
        if (error.code !== "target_effect_unverified") throw error;
        const state = await this.state();
        return { dispatched: true, receipt: error.details, state };
      }
    }
    return this._mutate("actions.keypress", { key, modifiers });
  }

  async wheelAt({ x, y, deltaX = 0, deltaY }) {
    if (![x, y, deltaX, deltaY].every(Number.isFinite)
      || x < 0 || x >= 1440 || y < 0 || y >= 900
      || (deltaX === 0 && deltaY === 0) || Math.abs(deltaX) > 3000 || Math.abs(deltaY) > 3000) {
      throw new TypeError("wheelAt requires a point within the 1440x900 logical viewport and finite, non-zero deltas within 3000 points");
    }
    if (this.protocol !== "hermes-native-window/1") {
      return this._mutate("actions.scroll_at", { x, y, delta_x: deltaX, delta_y: deltaY });
    }
    try {
      return await this._mutate("ui.wheel", { x, y, delta_x: deltaX, delta_y: deltaY });
    } catch (error) {
      if (error.code !== "target_effect_unverified") throw error;
      const state = await this.state();
      return { dispatched: true, receipt: error.details, state };
    }
  }

  async screenshot(filename) {
    if (this.protocol === "hermes-native-window/1") {
      const result = await this.request("screenshot.capture", {}, { mutation: true });
      if (filename === undefined) return result;
      const { writeFile } = await import("node:fs/promises");
      const { Buffer } = await import("node:buffer");
      await writeFile(filename, Buffer.from(result.png_base64, "base64"), { flag: "wx" });
      return { ...result, path: filename };
    }
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

  async click() {
    return this.page.protocol === "hermes-native-window/1"
      ? this._nativeAction(this.query, "click")
      : this.page._mutate("actions.click", { ...this.query });
  }
  async rightClick() { return this.page._mutate("actions.right_click", { ...this.query }); }
  async fill(text) {
    return this.page.protocol === "hermes-native-window/1"
      ? this._nativeAction(this.query, "set_value", { value: text })
      : this.page._mutate("actions.fill", { ...this.query, text });
  }
  async type(text) {
    if (this.page.protocol !== "hermes-native-window/1") return this.page._mutate("actions.type", { ...this.query, text });
    await this._nativeAction(this.query, "focus");
    return this._nativeAction(this.query, "replace_selected_text", { value: text });
  }
  async selectOption(option) {
    return this.page.protocol === "hermes-native-window/1"
      ? this._nativeSelectOption(option)
      : this.page._mutate("actions.select", { ...this.query, option });
  }
  async scroll({ deltaX = 0, deltaY }) {
    if (this.page.protocol === "hermes-native-window/1") return this._nativeScroll(this.query, deltaX, deltaY);
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
    if (this.page.protocol === "hermes-native-window/1") {
      const deadline = Date.now() + timeoutMs;
      while (Date.now() <= deadline) {
        const matches = await this._matches();
        if (matches.length === 1 && matches[0].visible && matches[0].enabled) return matches[0];
        await new Promise((resolve) => setTimeout(resolve, 100));
      }
      throw new HermesAgentError("wait_timeout", "Native UI control did not become uniquely visible and enabled");
    }
    return this.page.request("ui.wait_for", { ...this.query, timeout_ms: timeoutMs }, { timeoutMs: timeoutMs + 1_000 });
  }

  async _nativeAction(query, action, values = {}) {
    const root = await this.page.tree();
    const matches = [];
    const visit = (node) => {
      if (query.test_id
        ? node.automation_id === query.test_id
        : node.role === query.role && node.name === query.name) matches.push(node);
      for (const child of node.children ?? []) visit(child);
    };
    visit(root);
    if (matches.length !== 1) {
      throw new HermesAgentError(matches.length ? "target_ambiguous" : "target_not_found", `Native locator matched ${matches.length} controls; exactly one is required`);
    }
    const node = matches[0];
    if (!node.enabled) throw new HermesAgentError("target_disabled", "Native UI control is disabled");
    if (!node.actions?.includes(action)) throw new HermesAgentError("action_unsupported", `Native UI control does not support ${action}`);
    const locator = node.automation_id
      ? { automation_id: node.automation_id }
      : { role: node.role, name: node.name };
    return this.page._mutate("ui.action", { ...locator, action, ...values });
  }

  async _nativeScroll(query, deltaX, deltaY) {
    if (!Number.isFinite(deltaX) || !Number.isFinite(deltaY) || (deltaX === 0 && deltaY === 0)
      || Math.abs(deltaX) > 3000 || Math.abs(deltaY) > 3000) {
      throw new TypeError("native scroll deltas must be finite, non-zero and within 3000 points");
    }
    const root = await this.page.tree();
    const matches = [];
    const visit = (node) => {
      if (query.test_id
        ? node.automation_id === query.test_id
        : node.role === query.role && node.name === query.name) matches.push(node);
      for (const child of node.children ?? []) visit(child);
    };
    visit(root);
    if (matches.length !== 1) {
      throw new HermesAgentError(matches.length ? "target_ambiguous" : "target_not_found", `Native scroll locator matched ${matches.length} controls; exactly one is required`);
    }
    const node = matches[0];
    if (!node.enabled || !node.visible || !Array.isArray(node.bounds)
      || node.bounds[2] <= 0 || node.bounds[3] <= 0) {
      throw new HermesAgentError("target_not_actionable", "Native scroll target is disabled, outside the window viewport or has invalid bounds");
    }
    const locator = node.automation_id
      ? { automation_id: node.automation_id }
      : { role: node.role, name: node.name };
    try {
      return await this.page._mutate("ui.scroll", { ...locator, delta_x: deltaX, delta_y: deltaY });
    } catch (error) {
      if (error.code !== "target_effect_unverified") throw error;
      const state = await this.page.state();
      return { dispatched: true, receipt: error.details, state };
    }
  }

  async _nativeSelectOption(option) {
    if (typeof option !== "string" || !option.trim()) throw new TypeError("native selectOption requires an exact visible option label");
    const query = this.query;
    const matchQuery = (node) => query.test_id
      ? node.automation_id === query.test_id
      : node.role === query.role && node.name === query.name;
    const findMatches = (root) => {
      const matches = [];
      const visit = (node) => { if (matchQuery(node)) matches.push(node); for (const child of node.children ?? []) visit(child); };
      visit(root);
      return matches;
    };
    let root = await this.page.tree();
    let combos = findMatches(root);
    if (combos.length !== 1) throw new HermesAgentError(combos.length ? "target_ambiguous" : "target_not_found", `Native selector locator matched ${combos.length} controls`);
    if (combos[0].role !== "ComboBox") throw new HermesAgentError("action_unsupported", "Native selectOption requires a ComboBox locator");
    const collectOptions = (combo) => {
      const found = [];
      const visitOptions = (node) => {
        if (node.name === option && node.actions?.includes("click")) found.push(node);
        for (const child of node.children ?? []) visitOptions(child);
      };
      for (const child of combo.children ?? []) visitOptions(child);
      return found;
    };
    let options = collectOptions(combos[0]);
    if (options.length === 0) {
      await this._nativeAction(query, "click");
      root = await this.page.tree();
      combos = findMatches(root);
      if (combos.length !== 1) throw new HermesAgentError("target_not_found", "Native ComboBox disappeared after opening its option menu");
      options = collectOptions(combos[0]);
    }
    if (options.length !== 1) throw new HermesAgentError(options.length ? "option_ambiguous" : "option_not_found", `Native selector menu contains ${options.length} exact clickable options named ${option}`);
    const optionNode = options[0];
    const allMatches = [];
    const visitAll = (node) => { if (node.role === optionNode.role && node.name === optionNode.name) allMatches.push(node); for (const child of node.children ?? []) visitAll(child); };
    visitAll(root);
    if (allMatches.length !== 1) throw new HermesAgentError("option_ambiguous", `Native option locator matched ${allMatches.length} controls across the window`);
    const locator = optionNode.automation_id
      ? { automation_id: optionNode.automation_id }
      : { role: optionNode.role, name: optionNode.name };
    const receipt = await this.page._mutate("ui.action", { ...locator, action: "click" });
    const state = await this.page.state();
    const selectedField = query.name === "Interface mode" ? "interface_mode"
      : query.name === "Layout profile" ? "layout_profile"
      : null;
    const selected = selectedField ? state.snapshot?.[selectedField] : undefined;
    if (selected !== undefined) {
      const normalize = (value) => String(value).toLocaleLowerCase().replace(/[^a-z0-9]/g, "");
      if (normalize(selected) !== normalize(option)) {
        throw new HermesAgentError("selection_unverified", `Native selector state is ${selected}, expected ${option}`, undefined, { selected, requested: option, state });
      }
    }
    root = await this.page.tree();
    combos = findMatches(root);
    if (combos.length === 1 && collectOptions(combos[0]).length > 0) {
      await this.page.press("Escape");
      root = await this.page.tree();
      combos = findMatches(root);
      if (combos.length !== 1 || collectOptions(combos[0]).length > 0) {
        throw new HermesAgentError("selector_dismiss_unverified", "Native option was selected but its popup did not close after Escape");
      }
    }
    return { receipt, selected: selected ?? option, state };
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
