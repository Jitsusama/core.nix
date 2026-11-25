// pi as the host runs it reads a project's own settings, extensions, system
// prompt and MCP servers from the folder the host names in place of the
// project's .pi, such as a copy the host checked, which no session can
// write; and from the project's own .pi once named back. Unpatched, pi reads
// them from the project's .pi alone, so code a session writes there between
// the host's check and pi's load runs unchecked.
// Run as: node THIS PI_ROOT
import assert from "node:assert/strict";
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { test as it } from "node:test";
import { pathToFileURL } from "node:url";

const root = resolve(process.argv[2]);
// The coding agent sits beside its libraries in an installed tree, or is the
// tree itself where the package was installed from its own workspace.
const agentModule = (path) => {
  const found = [
    `${root}/node_modules/@earendil-works/pi-coding-agent/dist/${path}`,
    `${root}/packages/coding-agent/dist/${path}`,
    `${root}/dist/${path}`,
  ].find((candidate) => existsSync(candidate));
  assert.ok(found, `No coding agent ${path} under ${root}`);
  return import(pathToFileURL(found).href);
};
const { SettingsManager } = await agentModule("core/settings-manager.js");
const { DefaultResourceLoader } = await agentModule("core/resource-loader.js");
const { SessionManager } = await agentModule("core/session-manager.js");
const { createAgentSession } = await agentModule("core/sdk.js");
const { loadMcpConfig } = await agentModule("extensions/mcp/config.js");

const folder = mkdtempSync(join(tmpdir(), "pi-hosted-copy-"));
const cwd = join(folder, "project");
const agentDir = join(folder, "agent");
mkdirSync(agentDir, { recursive: true });
process.env.PI_CODING_AGENT_DIR = agentDir;

/**
 * A .pi at `base` whose every part says `name`: an extension registering a
 * command after it, a setting, a system prompt and an MCP server.
 */
const configDir = (base, name) => {
  const extension = join(base, "extensions", name);
  mkdirSync(extension, { recursive: true });
  writeFileSync(
    join(extension, "index.ts"),
    [
      "export default function (pi) {",
      `  pi.registerCommand(${JSON.stringify(`from-${name}`)}, { description: "", handler: async () => {} });`,
      "}",
      "",
    ].join("\n"),
  );
  writeFileSync(join(base, "settings.json"), JSON.stringify({ theme: name }));
  writeFileSync(join(base, "SYSTEM.md"), `read from ${name}\n`);
  writeFileSync(
    join(base, "mcp.json"),
    JSON.stringify({ mcpServers: { [name]: { command: "true" } } }),
  );
  return base;
};
const own = configDir(join(cwd, ".pi"), "tree");
const copy = configDir(join(folder, "copy"), "copy");

const commandsOf = (loader) =>
  loader
    .getExtensions()
    .extensions.flatMap((one) => [...one.commands.keys()])
    .sort();

it("reads a project's settings, extensions and system prompt from the folder named", async () => {
  const settingsManager = SettingsManager.create(cwd, agentDir, {
    projectTrusted: true,
    projectConfigDir: copy,
  });
  assert.equal(settingsManager.getProjectConfigDir(), copy);
  assert.equal(settingsManager.getProjectSettings().theme, "copy");
  const loader = new DefaultResourceLoader({ cwd, agentDir, settingsManager });
  await loader.reload();
  assert.deepEqual(commandsOf(loader), ["from-copy"], "the project's own .pi loaded");
  assert.equal(loader.getSystemPrompt()?.trim(), "read from copy");

  // Named back, the next reload reads the project's own .pi.
  settingsManager.setProjectConfigDir(own);
  await loader.reload();
  assert.equal(settingsManager.getProjectSettings().theme, "tree");
  assert.deepEqual(commandsOf(loader), ["from-tree"], "the copy loaded after it was named away");
  assert.equal(loader.getSystemPrompt()?.trim(), "read from tree");
});

it("reads a project's MCP servers from the folder named, and its own .pi without one", () => {
  const servers = (options) =>
    loadMcpConfig({ agentDir, cwd, projectTrusted: true, ...options }).servers.map(
      (server) => server.name,
    );
  assert.deepEqual(servers({ projectConfigDir: copy }), ["copy"]);
  assert.deepEqual(servers({}), ["tree"]);
});

it("tells a session's extensions which folder its project is read from", async () => {
  const settingsManager = SettingsManager.create(cwd, agentDir, {
    projectTrusted: true,
    projectConfigDir: copy,
  });
  const resourceLoader = new DefaultResourceLoader({
    cwd,
    agentDir,
    settingsManager,
    noExtensions: true,
    noSkills: true,
    noPromptTemplates: true,
    noThemes: true,
    noContextFiles: true,
  });
  await resourceLoader.reload();
  const { session } = await createAgentSession({
    cwd,
    agentDir,
    sessionManager: SessionManager.inMemory(cwd),
    settingsManager,
    resourceLoader,
    noTools: "all",
  });
  try {
    assert.equal(session.extensionRunner.createContext().getProjectConfigDir(), copy);
  } finally {
    session.dispose();
    rmSync(folder, { recursive: true, force: true });
  }
});
