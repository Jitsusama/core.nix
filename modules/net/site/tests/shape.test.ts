// How the network page lays the house out, and says its figures.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test as it } from "node:test";
import { type Device, networkOf } from "../model.ts";
import { study } from "../shape.ts";
import {
  deviceCommand,
  lasted,
  linkSpeed,
  possessive,
  rank,
  rate,
  shellWord,
  slug,
} from "../words.ts";
import { told } from "./house.ts";

function shaped() {
  const { watched, kinds } = told();
  const network = networkOf(watched, kinds);
  assert.ok(network, "the whole house was told, so the page draws it");
  return { network, shape: study(network) };
}

it("the floors go top first, each eero's devices on its own", () => {
  const { shape } = shaped();
  assert.deepEqual(
    shape.floors.map((floor) => [floor.point.name, floor.devices.map((device) => device.name)]),
    [
      ["Upstairs", ["Kitchen Printer"]],
      ["Basement", []],
    ],
  );
});

it("the router is the gateway, and the modem sits on no floor of its own", () => {
  const { shape } = shaped();
  assert.equal(shape.gateway?.id, "eero:1");
  assert.equal(shape.modem?.id, "modem");
  assert.ok(shape.floors.every((floor) => !floor.devices.includes(shape.modem ?? floor.point)));
});

it("a device hangs from the first hop of its way out, and routes cross where net counted", () => {
  const { shape } = shaped();
  const printer = shape.byId.get("mac:00:00:5e:00:53:10");
  assert.equal(printer === undefined ? undefined : shape.via(printer), "eero:2");
  assert.equal(shape.crossing("modem", "eero:1"), 3);
  assert.equal(shape.routed, 3);
});

it("a floor's place comes from the words in its eero's name", () => {
  assert.deepEqual(["Attic", "Kitchen", "Den", "Basement"].map(rank), [0, 1, 2, 3]);
});

it("a measured rate reads as net says it, in three figures", () => {
  // net's figures_tests.rs reads the same cases, so the panel and net's
  // concerns can't round one rate two ways.
  const cases: { mbps: number; says: string }[] = JSON.parse(
    readFileSync(new URL("rates.json", import.meta.url), "utf8"),
  );
  assert.ok(cases.length > 0, "the shared cases are there");
  assert.deepEqual(
    cases.map((c) => rate(c.mbps)),
    cases.map((c) => c.says),
  );
});

it("a rate of no number is none", () => {
  assert.deepEqual([Number.NaN, Number.POSITIVE_INFINITY].map(rate), ["0 b/s", "0 b/s"]);
});

it("a link's speed is a name, said with only the digits it has", () => {
  assert.deepEqual([1000, 2500, 10_000, 866, 1201, 2402, 72, 72.2, 100, 999.999].map(linkSpeed), [
    "1 Gb/s",
    "2.5 Gb/s",
    "10 Gb/s",
    "866 Mb/s",
    "1201 Mb/s",
    "2402 Mb/s",
    "72 Mb/s",
    "72.2 Mb/s",
    "100 Mb/s",
    "1 Gb/s",
  ]);
});

it("a span says the largest unit or two", () => {
  assert.deepEqual([42, 600, 3 * 3600 + 120, 30 * 3600, 50 * 3600].map(lasted), [
    "42 seconds",
    "10 minutes",
    "3 hours, 2 minutes",
    "30 hours",
    "2 days, 2 hours",
  ]);
});

it("a name becomes an address's fragment, and owns things as English writes it", () => {
  assert.equal(slug("Joel's Café TV"), "joel-s-cafe-tv");
  assert.equal(possessive("Upstairs"), "Upstairs'");
  assert.equal(possessive("Basement"), "Basement's");
});

it("a name in a command is quoted so pasting it runs nothing else", () => {
  for (const [name, typed] of [
    ["Study", "Study"],
    ["tv_2@home%5+=a:b,c./d-e", "tv_2@home%5+=a:b,c./d-e"],
    ["Main Floor", "'Main Floor'"],
    ["lamp;reboot", "'lamp;reboot'"],
    ["$(reboot)", "'$(reboot)'"],
    ["`reboot`", "'`reboot`'"],
    ["Joel's | sh", "'Joel'\\''s | sh'"],
    ["=ls", "'=ls'"],
    ["a&b", "'a&b'"],
    ["*?", "'*?'"],
    ["~", "'~'"],
    ["hi!", "'hi!'"],
    ["<in >out", "'<in >out'"],
    ["back\\slash", "'back\\slash'"],
    ["", "''"],
  ]) {
    assert.equal(
      shellWord(name),
      typed,
      `${JSON.stringify(name)} wasn't one word that runs nothing`,
    );
  }
});

/** A device net read, with only what naming it at a terminal reads. */
function named(name: string, id: string, addresses: readonly string[] = []): Device {
  return {
    id,
    name,
    roles: [],
    addresses,
    macs: [],
    source: undefined,
    vendor: undefined,
    model: undefined,
    hardware: undefined,
    firmware: undefined,
    serial: undefined,
    upS: undefined,
    readAt: undefined,
    path: undefined,
    radios: [],
    ports: [],
    speed: undefined,
    modem: undefined,
  };
}

it("a device's command names it by a name that pastes as it reads and finds it alone", () => {
  const id = "mac:02:00:00:00:00:01";
  const byId = `net devices ${id}`;
  // net takes a word as an id, then an address, then a name in ASCII
  // case, so each of these finds another device or more than one.
  const others = [
    named("Study", "mac:02:00:00:00:00:02", ["10.0.0.2"]),
    named("Den", "lamp-id"),
    named("ÉCOLE", "mac:02:00:00:00:00:03"),
    named("Caf\u00e9", "mac:02:00:00:00:00:04"),
    named("Lo\u00adft", "mac:02:00:00:00:00:05"),
    named("kitchen", "mac:02:00:00:00:00:06"),
    named("Attic\u2063", "mac:02:00:00:00:00:07"),
    named("stra\u00dfe", "mac:02:00:00:00:00:08"),
    named("\u039f\u0394\u039f\u03a3 hall", "mac:02:00:00:00:00:09"),
    named("Back\u00a0\u00a0porch", "mac:02:00:00:00:00:0a"),
    named("Ga\ufeffrage", "mac:02:00:00:00:00:0b"),
    named("Hall \u2063 way", "mac:02:00:00:00:00:0c"),
    named("Tool\tshed", "mac:02:00:00:00:00:0d"),
    named("Sun\u3164room", "mac:02:00:00:00:00:0e"),
    named("Mud\u2800room", "mac:02:00:00:00:00:0f"),
    named("Wine\u115fcellar", "mac:02:00:00:00:00:10"),
    named(" Shed ", "mac:02:00:00:00:00:11"),
    named("Pan\uffa0try", "mac:02:00:00:00:00:12"),
    named("Bath\nroom", "mac:02:00:00:00:00:13"),
    named("Store\vroom", "mac:02:00:00:00:00:15"),
    named("Play\froom", "mac:02:00:00:00:00:16"),
    named("Bed\rroom", "mac:02:00:00:00:00:17"),
    named("\u03aa\u0301 room", "mac:02:00:00:00:00:14"),
  ];
  for (const [name, command] of [
    ["Hall", "net devices Hall"],
    ["lamp;reboot", "net devices 'lamp;reboot'"],
    ["Main Floor", "net devices 'Main Floor'"],
    ["Salle de séjour 📺", "net devices 'Salle de séjour 📺'"],
    ["Joel's \u2764\ufe0f phone", "net devices 'Joel'\\''s \u2764\ufe0f phone'"],
    ["\u{1f469}\u200d\u{1f4bb} desk", "net devices '\u{1f469}\u200d\u{1f4bb} desk'"],
    // What the kit keeps to shape a word, mid-word: a soft hyphen, a
    // non-joiner, a flag's tags, Hangul's vowel filler, a Mongolian
    // selector and an ideograph's supplementary selector.
    ["hall\u00adway", "net devices 'hall\u00adway'"],
    [
      "\u0645\u06cc\u200c\u062e\u0648\u0627\u0647\u0645",
      "net devices '\u0645\u06cc\u200c\u062e\u0648\u0627\u0647\u0645'",
    ],
    [
      "\u{1f3f4}\u{e0067}\u{e0062}\u{e0073}\u{e0063}\u{e0074}\u{e007f} den",
      "net devices '\u{1f3f4}\u{e0067}\u{e0062}\u{e0073}\u{e0063}\u{e0074}\u{e007f} den'",
    ],
    ["\u1100\u1160 room", "net devices '\u1100\u1160 room'"],
    ["\u1100\u1161 room", "net devices '\u1100\u1161 room'"],
    ["\u1820\u180b desk", "net devices '\u1820\u180b desk'"],
    ["\u845b\u{e0100} hall", "net devices '\u845b\u{e0100} hall'"],
    // Each kept range's far end, and the selectors and separator between.
    [
      "\u1820\u180c\u1821\u180d\u1822\u180e\u1823\u180f",
      "net devices '\u1820\u180c\u1821\u180d\u1822\u180e\u1823\u180f'",
    ],
    ["\u845b\ufe00 \u845b\u{e01ef}", "net devices '\u845b\ufe00 \u845b\u{e01ef}'"],
    ["\u{1f3f4}\u{e0020}\u{e007f} den", "net devices '\u{1f3f4}\u{e0020}\u{e007f} den'"],
    // Beside Hangul's ranges that take no cell: the letter just past the
    // first, the last syllable, and the code point just below the second.
    ["a \u1200x", "net devices 'a \u1200x'"],
    ["a \ud7a3x", "net devices 'a \ud7a3x'"],
    ["a \ud7afx", "net devices 'a \ud7afx'"],
    ["study", byId],
    ["10.0.0.2", byId],
    ["lamp-id", byId],
    ["--json", byId],
    ["", byId],
    ["Main  Floor", byId],
    ["lamp\u001b[201~reboot", byId],
    ["tab\there", byId],
    ["two\nlines", byId],
    ["next\u2028line", byId],
    ["para\u2029graph", byId],
    ["\u202eevil", byId],
    ["zero\u200bwidth", byId],
    ["no\u00a0break", byId],
    ["braille\u2800blank", byId],
    ["hangul\u115ffiller", byId],
    ["private\ue000use", byId],
    ["lone\ud800half", byId],
    ["\u200dalone", byId],
    ["joined \ufe0fbare", byId],
    ["\u0301accent", byId],
    ["a \u0301accent", byId],
    ["filler \u1160bare", byId],
    ["a \u1161vowel", byId],
    ["\u11a8final", byId],
    ["x \ud7b0vowel", byId],
    ["a \u11ffx", byId],
    ["x \ud7ffv", byId],
    // Just past each kept range: a tag that opens a language, a tag and a
    // selector nothing assigns.
    ["a\u{e0001}b", byId],
    ["a\u{e0080}b", byId],
    ["a\u{e01f0}b", byId],
    // Another's name once what shapes it is set aside.
    ["Den\u200d", byId],
    ["D\u00aden", byId],
    ["Loft", byId],
    // Another's id or address once what shapes it is set aside, and
    // another's name as Unicode composes it.
    ["lamp\u200d-id", byId],
    ["10.0.0\u00ad.2", byId],
    ["Cafe\u0301", byId],
    // A joiner between a letter and its mark, and a capital however it's
    // written: with its mark, composed, or as the Kelvin sign.
    ["Cafe\u200d\u0301", byId],
    ["CAFE\u0301", byId],
    ["CAF\u00c9", byId],
    // A small letter net tells from its capital, though a person doesn't.
    ["école", byId],
    ["\u212acafe", "net devices '\u212acafe'"],
    ["\u212aitchen", byId],
    // Another's name read past what draws nothing, with its blanks one
    // space, a wide letter as the letter, and a case that changes length.
    ["attic", byId],
    ["back porch", byId],
    ["\uff24en", byId],
    ["STRASSE", byId],
    ["STRA\u1e9eE", byId],
    ["\u03bf\u03b4\u03bf\u03c3 hall", byId],
    ["garage", byId],
    ["hall way", byId],
    ["tool shed", byId],
    ["sun room", byId],
    ["mud room", byId],
    ["wine cellar", byId],
    ["shed", byId],
    // Hangul's halfwidth filler reads as a blank, as does each control
    // from a line feed to a carriage return.
    ["pan try", byId],
    ["bath room", byId],
    ["store room", byId],
    ["play room", byId],
    ["bed room", byId],
    // A letter its case leaves apart from its marks, which only the last
    // composing brings together: ΐ as the small of Ϊ and its accent.
    ["\u0390 room", byId],
    // And names a letter apart from another's, which are kept.
    ["Cafe", "net devices Cafe"],
    ["Dens", "net devices Dens"],
    ["strase", "net devices strase"],
    // An id net gives several, which it reads as the first holding it, and
    // one that reads as it to a person, though net takes it as a name.
    ["mac:?", byId],
    ["EERO:unknown", byId],
  ] as const) {
    // A copy, as a device read again is, so the device is known by its id.
    const device = named(name, id);
    assert.equal(
      deviceCommand({ ...device }, [device, ...others]),
      command,
      `${JSON.stringify(name)} was named wrongly`,
    );
  }
});

it("a device is named by its name, else its address, else its id, or by none", () => {
  const shared = named("Phone", "mac:02:00:00:00:00:02", ["10.0.0.2"]);
  const network = (device: Device): Device[] => [device, shared, named("Hub", "hub", ["10.0.0.9"])];
  for (const [device, command, why] of [
    [named("Phone", "mac:?", ["10.0.0.4"]), "net devices 10.0.0.4", "with its own address"],
    [
      named("Hall", "mac:02:00:00:00:00:09", ["10.0.0.6"]),
      "net devices Hall",
      "with a name and an address that each find it",
    ],
    [
      named("Phone", "mac:02:00:00:00:00:0a", ["hub"]),
      "net devices mac:02:00:00:00:00:0a",
      "with another's id as its address",
    ],
    [named("Phone", "mac:?", ["mac:?"]), undefined, "with its own shared id as its address"],
    [
      named("Phone", "mac:02:00:00:00:00:0b", ["mac:?"]),
      "net devices mac:02:00:00:00:00:0b",
      "with a shared id as its address",
    ],
    [named("Phone", "mac:?", ["10.0.0.9"]), undefined, "with another's address"],
    [named("Phone", "mac:?"), undefined, "with the id net gives every client without one"],
    [named("Phone", "eero:unknown"), undefined, "with the id net gives every eero without one"],
    [
      named("Phone", "mac:02:00:00:00:00:08", ["10.0.0.5"]),
      "net devices 10.0.0.5",
      "with an address and an id",
    ],
    [
      named("Phone", "mac:02:00:00:00:00:07"),
      "net devices mac:02:00:00:00:00:07",
      "with only its id",
    ],
  ] as const) {
    assert.equal(
      deviceCommand(device, network(device)),
      command,
      `a device ${why} was named wrongly`,
    );
  }
});
