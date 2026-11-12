// Reads the Vimium settings core.nix writes as Vimium restores a backup:
// its own parser (background_scripts/commands.js) takes the key lines
// after its default keys, as it lays them, so a line it refuses, or a key
// that doesn't do what the key language says, fails, naming it. Its own
// H, L, x, f and F are held too, which stay beside the language's, and its
// own reading of the exclusion rules (background_scripts/exclusions.js)
// finds it off on GitHub and every Google page with keys of its own, and
// on everywhere else. Run as
// `node chrome-vimium-takes-the-keys.mjs <Vimium's source> <vimium.json>`.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

const [source, settingsFile] = process.argv.slice(2);

// The one helper of Vimium's the parser asks for, from lib/utils.js, and
// a browser with no APIs, which its list of commands reads a limit from.
globalThis.Utils = {
  keyBy: (array, key) => Object.fromEntries(array.map((item) => [item[key], item])),
};
globalThis.chrome = {};
const commands = join(source, "background_scripts", "commands.js");
const { KeyMappingsParser, defaultKeyMappings } = await import(pathToFileURL(commands).href);

// What each key does, as the language has it, by Vimium's command, then
// Vimium's own keys, kept beside them.
const wanted = [
  ["j", "scrollDown"],
  ["k", "scrollUp"],
  ["h", "scrollLeft"],
  ["l", "scrollRight"],
  ["<c-e>", "scrollDown"],
  ["<c-y>", "scrollUp"],
  ["<c-d>", "scrollPageDown"],
  ["<c-u>", "scrollPageUp"],
  ["<c-f>", "scrollFullPageDown"],
  ["<c-b>", "scrollFullPageUp"],
  ["gg", "scrollToTop"],
  ["G", "scrollToBottom"],
  ["<c-o>", "goBack"],
  ["<c-i>", "goForward"],
  ["/", "enterFindMode"],
  ["n", "performFind"],
  ["N", "performBackwardsFind"],
  ["yy", "copyCurrentUrl"],
  ["ZZ", "removeTab"],
  ["H", "goBack"],
  ["L", "goForward"],
  ["x", "removeTab"],
  ["f", "LinkHints.activateMode"],
  ["F", "LinkHints.activateModeToOpenInNewTab"],
];

const settings = JSON.parse(readFileSync(settingsFile, "utf8"));
assert.equal(typeof settings.keyMappings, "string", "the settings hold key lines");
// Without a version, Vimium takes the backup for one from before 2.0 and
// reads each value as JSON written in a string.
assert.equal(typeof settings.settingsVersion, "string", "the settings say their version");

const defaults = Object.entries(defaultKeyMappings)
  .map(([key, command]) => `map ${key} ${command}`)
  .join("\n");
const parsed = KeyMappingsParser.parse(`${defaults}\n${settings.keyMappings}`, false);
assert.deepEqual(parsed.validationErrors, [], "Vimium refuses none of the lines");

// The parser keeps each line's key as written, so a key is found by its
// keys as Vimium reads them, whatever the case of its modifier.
const bound = new Map(
  Object.values(parsed.keyToRegistryEntry).map((entry) => [
    entry.keySequence.join(""),
    entry.command,
  ]),
);
const wrong = wanted
  .filter(([key, command]) => bound.get(key) !== command)
  .map(([key, command]) => `${key} does ${bound.get(key) ?? "nothing"}, not ${command}`);
assert.deepEqual(wrong, [], "each key does what it should");

// Where Vimium is off, leaving the page its own keys, and where it's on.
globalThis.Settings = {
  get: () => settings.exclusionRules,
  addEventListener: () => undefined,
};
const exclusions = join(source, "background_scripts", "exclusions.js");
const { isEnabledForUrl } = await import(pathToFileURL(exclusions).href);
const off = [
  "https://mail.google.com/mail/u/0/#inbox",
  "https://calendar.google.com/calendar/r/week",
  "https://meet.google.com/abc-defg-hij",
  "https://docs.google.com/document/d/1/edit",
  "https://docs.google.com/spreadsheets/d/1/edit",
  "https://docs.google.com/presentation/d/1/edit",
  "https://drive.google.com/drive/my-drive",
  "https://github.com/philc/vimium",
];
const on = ["https://www.google.com/search?q=vimium", "https://en.wikipedia.org/wiki/Vim"];
const misjudged = [
  ...off.filter((url) => isEnabledForUrl(url).isEnabledForUrl).map((url) => `on at ${url}`),
  ...on.filter((url) => !isEnabledForUrl(url).isEnabledForUrl).map((url) => `off at ${url}`),
];
assert.deepEqual(misjudged, [], "Vimium is off only where a page has keys of its own");
