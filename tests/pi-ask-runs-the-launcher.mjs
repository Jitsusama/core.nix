// pi-ask's launcher tool (modules/pi/ask-launcher.ts, the first argument),
// over a stand-in qs answering as the launcher's catalogue does. Listing
// gives each entry's id, place, words and what its row says now, marks a
// disruptive one, and keeps to a match; running one, with runWith for one
// that takes words, passes on what the launcher did, that a
// disruptive one was only offered for Return, or why it couldn't; and a
// launcher that doesn't answer is said so. pi's own library, which the tool
// takes its schema builder from, is a stand-in too.
import assert from "node:assert/strict";
import { chmodSync, copyFileSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const work = mkdtempSync(join(tmpdir(), "ask-launcher-"));
copyFileSync(process.argv[2], join(work, "ask-launcher.ts"));

const library = join(work, "node_modules", "@earendil-works", "pi-ai");
mkdirSync(library, { recursive: true });
writeFileSync(join(library, "package.json"), JSON.stringify({ name: "@earendil-works/pi-ai", type: "module" }));
writeFileSync(
  join(library, "index.js"),
  "const schema = (kind) => (...args) => ({ kind, args });\n" +
    "export const Type = { Object: schema('object'), Union: schema('union'), Literal: schema('literal'), " +
    "Optional: schema('optional'), String: schema('string') };\n",
);

const catalogue = [
  { id: "app:kitty", kind: "place", in: "Programs", words: "Kitty", disruptive: false },
  {
    id: "category:computer›category:energy›session:Reboot",
    kind: "action",
    in: "Machine › Power",
    words: "Reboot",
    disruptive: true,
  },
  {
    id: "category:computer›setting:brightness",
    kind: "action",
    in: "Machine › Screen",
    words: "Brightness",
    says: "60%",
    disruptive: false,
    args: [{ name: "percent", words: "a percentage" }],
  },
];
const asked = join(work, "asked");
const qs = join(work, "qs");
writeFileSync(
  qs,
  `#!/bin/sh
echo "$*" >> "${asked}"
[ -e "${work}/down" ] && { echo "no running instance" >&2; exit 1; }
case "$4" in
  list) cat "${work}/catalogue" ;;
  run)
    case "$5" in
      app:kitty) echo '{"done":true,"said":"Opened Kitty"}' ;;
      category:computer›category:energy›session:Reboot) echo '{"done":false,"offered":true,"said":"Reboot?"}' ;;
      *) echo '{"error":"no entry by that id"}' ;;
    esac ;;
  runWith)
    case "$5" in
      category:computer›setting:brightness) echo "{\\"done\\":true,\\"said\\":\\"Brightness at $6%\\"}" ;;
      *) echo '{"error":"no entry by that id"}' ;;
    esac ;;
esac
`,
);
chmodSync(qs, 0o755);
writeFileSync(join(work, "catalogue"), JSON.stringify(catalogue));
process.env.ASK_LAUNCHER = qs;

let tool;
const { default: register } = await import(join(work, "ask-launcher.ts"));
register({ registerTool: (registered) => (tool = registered) });
assert.equal(tool.name, "launcher");
assert.match(tool.description, /never that it happened/);

const said = async (params) => (await tool.execute("call", params, undefined)).content[0].text;

const everything = await said({ action: "list" });
assert.match(everything, /^app:kitty: Programs › Kitty$/m);
assert.match(
  everything,
  /^category:computer›category:energy›session:Reboot: Machine › Power › Reboot \[disruptive: offered for Return, not done\]$/m,
);
assert.match(everything, /Machine › Screen › Brightness \(now 60%\) \(takes a percentage\)$/m);

const power = await said({ action: "list", match: "power" });
assert.match(power, /Reboot/);
assert.doesNotMatch(power, /Kitty/);
assert.equal(await said({ action: "list", match: "toaster" }), 'Nothing in the launcher matches "toaster".');

assert.equal(await said({ action: "run", id: "app:kitty" }), "Done: Opened Kitty");
assert.equal(await said({ action: "run", id: "category:computer›setting:brightness", arg: "40" }), "Done: Brightness at 40%");
assert.match(await said({ action: "run", id: "category:computer›category:energy›session:Reboot" }), /^Not done yet: .*a row \(Reboot\?\) and Return does it/);
assert.equal(await said({ action: "run", id: "nothing" }), "Not done: no entry by that id");
assert.equal(await said({ action: "run" }), "Name the id to run, as list gives it.");

const calls = readFileSync(asked, "utf8").trim().split("\n");
assert.equal(calls[0], "ipc call launcher list");
assert.ok(calls.includes("ipc call launcher runWith category:computer›setting:brightness 40"));
assert.ok(calls.includes("ipc call launcher run app:kitty"));

writeFileSync(join(work, "down"), "");
await assert.rejects(said({ action: "list" }), /The launcher didn't answer: no running instance/);
