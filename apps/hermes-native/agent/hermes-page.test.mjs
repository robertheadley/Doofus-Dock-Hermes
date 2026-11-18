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

const fakeNativeProtocol = String.raw`
const readline = require("node:readline");
let generation = 0;
let paletteOpen = false;
let selectorOpen = false;
let route = "/";
let interfaceMode = "advanced";
let scrollOffset = 0;
const makeTree = () => {
  const nodes = [
    { id: 0, parent_id: null, children: [1,2,3,4], role: "Window", name: "Hermes", automation_id: "", enabled: true, offscreen: false, focused: false, bounds_valid: true, bounds: {x:-1000,y:-800,width:1440,height:900}, actions: [] },
    { id: 1, parent_id: 0, children: [], role: "Button", name: "Capabilities", automation_id: "route.capabilities", enabled: true, offscreen: false, focused: false, bounds: {x:-990,y:-770,width:120,height:32}, actions: ["click"] },
    { id: 2, parent_id: 0, children: [], role: "Edit", name: "Search sessions", automation_id: "session.search", enabled: true, offscreen: false, focused: false, bounds: {x:-980,y:-720,width:200,height:30}, actions: ["focus","set_value","replace_selected_text"] },
    { id: 3, parent_id: 0, children: selectorOpen ? [5] : [], role: "ComboBox", name: "Interface mode", automation_id: "interface.mode", enabled: true, offscreen: false, focused: false, bounds: {x:-900,y:-770,width:150,height:32}, actions: ["click"] },
    { id: 4, parent_id: 0, children: [], role: "Group", name: "Plugin catalog list", automation_id: "plugin.list", enabled: true, offscreen: false, focused: false, bounds_valid: true, bounds: {x:-500,y:-300,width:800,height:400}, actions: [] },
  ];
  if (selectorOpen) nodes.push(
    { id: 5, parent_id: 3, children: [6,7], role: "List", name: null, automation_id: "", enabled: true, offscreen: false, focused: false, bounds_valid: false, bounds: {x:0,y:0,width:0,height:0}, actions: [] },
    { id: 6, parent_id: 5, children: [], role: "Button", name: "Advanced", automation_id: "mode.advanced", enabled: true, offscreen: false, focused: false, bounds: {x:-900,y:-730,width:150,height:32}, actions: ["click"] },
    { id: 7, parent_id: 5, children: [], role: "Button", name: "Simple", automation_id: "mode.simple", enabled: true, offscreen: false, focused: false, bounds: {x:-900,y:-698,width:150,height:32}, actions: ["click"] },
  );
  return { root: 0, node_count: nodes.length, truncated: false, nodes };
};
process.stdout.write(JSON.stringify({version:1,type:"ready",protocol:"hermes-native-window/1",state_generation:generation})+"\n");
readline.createInterface({input:process.stdin}).on("line", line => {
  const request = JSON.parse(line);
  let response;
  if (request.method === "ui.tree") response = {version:1,id:request.id,status:"succeeded",result:{state_generation:generation,tree:makeTree()}};
  else if (request.method === "state.get") response = {version:1,id:request.id,status:"succeeded",result:{state_generation:generation,route:{path:route},snapshot:{command_palette:{open:paletteOpen},interface_mode:interfaceMode,scroll_offset:scrollOffset}}};
  else if (request.method === "ui.action") {
    if (request.params.expected_state_generation !== generation) response = {version:1,id:request.id,status:"failed",error:{code:"stale_generation",state_generation:generation,message:"stale"}};
    else { generation++; if (request.params.automation_id === "route.capabilities") route = "/capabilities"; if (request.params.automation_id === "interface.mode") selectorOpen = !selectorOpen; if (request.params.automation_id === "mode.simple") interfaceMode = "simple"; if (request.params.automation_id === "mode.advanced") interfaceMode = "advanced"; response = {version:1,id:request.id,status:"succeeded",result:{state_generation_after:generation,action:request.params.action}}; }
  } else if (request.method === "ui.keypress") {
    if (request.params.expected_state_generation !== generation) response = {version:1,id:request.id,status:"failed",error:{code:"stale_generation",state_generation:generation,message:"stale"}};
    else { generation++; if (request.params.key.toUpperCase() === "P") paletteOpen = true; if (request.params.key.toUpperCase() === "ESCAPE") { paletteOpen = false; selectorOpen = false; } response = {version:1,id:request.id,status:"failed",error:{code:"target_effect_unverified",state_generation:generation,input_dispatched:true,message:"verify state"}}; }
  } else if (request.method === "ui.scroll") {
    if (request.params.expected_state_generation !== generation) response = {version:1,id:request.id,status:"failed",error:{code:"stale_generation",state_generation:generation,message:"stale"}};
    else { generation++; scrollOffset += request.params.delta_y; response = {version:1,id:request.id,status:"failed",error:{code:"target_effect_unverified",state_generation:generation,input_dispatched:true,delta_y:request.params.delta_y,message:"verify state"}}; }
  } else if (request.method === "ui.wheel") {
    if (request.params.expected_state_generation !== generation) response = {version:1,id:request.id,status:"failed",error:{code:"stale_generation",state_generation:generation,message:"stale"}};
    else { generation++; scrollOffset += request.params.delta_y; response = {version:1,id:request.id,status:"failed",error:{code:"target_effect_unverified",state_generation:generation,input_dispatched:true,x:request.params.x,y:request.params.y,delta_y:request.params.delta_y,message:"verify state"}}; }
  } else if (request.method === "screenshot.capture") response = {version:1,id:request.id,status:"succeeded",result:{state_generation:generation,mime_type:"image/png",png_base64:"aGVsbG8=",sha256:"test"}};
  else response = {version:1,id:request.id,status:"succeeded",result:{state_generation:generation}};
  process.stdout.write(JSON.stringify(response)+"\n");
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

test("HermesPage uses locators and state-verified keypresses with the native protocol", async () => {
  const page = await HermesPage.launch({
    command: execPath,
    args: ["-e", fakeNativeProtocol],
    cwd: process.cwd(),
  });
  try {
    const capabilities = page.getByRole("Button", { name: "Capabilities" });
    assert.equal(await capabilities.count(), 1);
    assert.equal(await capabilities.isVisible(), true);
    assert.deepEqual(await capabilities.boundingBox(), { x: 10, y: 30, width: 120, height: 32 });
    await capabilities.click();
    assert.equal((await page.state()).route.path, "/capabilities");

    const search = page.getByTestId("session.search");
    assert.equal(await search.isEnabled(), true);
    await search.fill("native");
    await search.type(" typing");
    const mode = await page.getByRole("ComboBox", { name: "Interface mode" }).selectOption("Simple");
    assert.equal(mode.selected, "simple");
    const advanced = await page.getByRole("ComboBox", { name: "Interface mode" }).selectOption("Advanced");
    assert.equal(advanced.selected, "advanced");
    const scrolled = await page.getByTestId("plugin.list").scroll({ deltaY: -80 });
    assert.equal(scrolled.dispatched, true);
    assert.equal(scrolled.receipt.delta_y, -80);
    assert.equal(scrolled.state.snapshot.scroll_offset, -80);
    const wheel = await page.wheelAt({ x: 600, y: 400, deltaY: -120 });
    assert.equal(wheel.dispatched, true);
    assert.equal(wheel.receipt.x, 600);
    assert.equal(wheel.receipt.delta_y, -120);
    assert.equal(wheel.state.snapshot.scroll_offset, -200);
    const opened = await page.press("P", ["CTRL"]);
    assert.equal(opened.dispatched, true);
    assert.equal(opened.receipt.input_dispatched, true);
    assert.equal(opened.state.snapshot.command_palette.open, true);
    const closed = await page.press("ESCAPE");
    assert.equal(closed.state.snapshot.command_palette.open, false);
    assert.equal((await page.screenshot()).png_base64, "aGVsbG8=");
  } finally {
    await page.close();
  }
});
