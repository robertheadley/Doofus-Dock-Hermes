import test from "node:test";
import assert from "node:assert/strict";
import { execPath } from "node:process";
import { HermesAgentError, HermesPage } from "./hermes-page.mjs";

const fakeProtocol = String.raw`
const readline = require("node:readline");
let generation = 0;
let requestCount = 0;
const tree = { role: "Window", name: "Hermes", visible: true, enabled: true, bounds: [0,0,1440,900], children: [
  { role: "Button", name: "Capabilities", test_id: "route.capabilities", visible: true, enabled: true, bounds: [8,8,120,32], children: [] }
]};
process.stdout.write(JSON.stringify({version:1,type:"ready",state_generation:generation})+"\n");
readline.createInterface({input:process.stdin}).on("line", line => {
  requestCount += 1;
  const request = JSON.parse(line);
  let result = {};
  if (request.method === "ui.tree") result = tree;
  if (request.method === "state.get") result = {state_generation:generation,request_count:requestCount};
  if (request.method === "ui.wait_for") result = {role:request.params.role,name:request.params.name,visible:true,state_generation:generation};
  if (request.method.startsWith("actions.")) {
    if (request.params.expected_state_generation !== generation) {
      process.stdout.write(JSON.stringify({version:1,id:request.id,status:"failed",error:{code:"stale_state",message:"stale"}})+"\n");
      return;
    }
    generation += 1;
    result = {state_generation_before:generation-1,state_generation_after:generation,command:request.method};
  }
  if (request.method === "screenshots.capture" && request.params.filename === "hang.png") return;
  if (request.method === "screenshots.capture") result = {path:request.params.filename};
  process.stdout.write(JSON.stringify({version:1,id:request.id,status:"succeeded",result})+"\n");
  if (request.method === "protocol.close") process.exit(0);
});
`;

test("HermesPage drives semantic locators and tracks action generations", async () => {
  const page = await HermesPage.launch({
    command: execPath,
    args: ["-e", fakeProtocol],
    cwd: process.cwd(),
  });
  try {
    const capabilities = page.getByRole("Button", { name: "Capabilities" });
    assert.equal(await capabilities.count(), 1);
    assert.equal(await capabilities.isVisible(), true);
    assert.equal(await capabilities.isEnabled(), true);
    assert.deepEqual(await capabilities.boundingBox(), { x: 8, y: 8, width: 120, height: 32 });
    assert.equal((await capabilities.waitFor()).visible, true);

    const receipt = await page.getByTestId("route.capabilities").click();
    assert.equal(receipt.state_generation_before, 0);
    assert.equal(receipt.state_generation_after, 1);
    const fillReceipt = await page.getByRole("TextInput", { name: "Search" }).fill("toolsets");
    assert.equal(fillReceipt.state_generation_before, 1);
    assert.equal(fillReceipt.state_generation_after, 2);
    assert.deepEqual(await page.screenshot("agent-check.png"), { path: "agent-check.png" });
  } finally {
    await page.close();
  }
});

test("HermesPage surfaces stale-generation refusals without replaying the mutation", async () => {
  const page = await HermesPage.launch({
    command: execPath,
    args: ["-e", fakeProtocol],
    cwd: process.cwd(),
  });
  try {
    page.generation = 7;
    const target = page.getByRole("Button", { name: "Capabilities" });
    await assert.rejects(target.click(), (error) => {
      assert.ok(error instanceof HermesAgentError);
      assert.equal(error.code, "stale_state");
      return true;
    });
    assert.equal((await page.state()).request_count, 2);
  } finally {
    await page.close();
  }
});

test("HermesPage marks timed-out screenshot writes unknown and does not replay them", async () => {
  const page = await HermesPage.launch({
    command: execPath,
    args: ["-e", fakeProtocol],
    cwd: process.cwd(),
    requestTimeoutMs: 25,
  });
  try {
    await assert.rejects(page.screenshot("hang.png"), (error) => {
      assert.ok(error instanceof HermesAgentError);
      assert.equal(error.code, "request_timeout_outcome_unknown");
      assert.match(error.message, /outcome is unknown and was not retried/);
      return true;
    });
    assert.equal((await page.state()).request_count, 2);
  } finally {
    await page.close();
  }
});
