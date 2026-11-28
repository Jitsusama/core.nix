// The kit's ask row reads a look as the launcher's For You does, held
// against the launcher's own code rather than against what ask.ts says it
// copied, so a rule changed on either side fails here. Looks.js's `look`,
// `quieted` and `asksNothing` and Now.qml's own lookOffers and followsOf
// decide, over looks at each boundary and of every kind and state, which a
// page reads, which wait, what each offers and what follows each, and when
// it stops; LookForm.js's wordsOf, what the second form leads with; Now.qml's
// lookDrawer, which kinds it links, at a web address only, the page's own
// rule; LookForm.js's actOf decides its act, beside the launcher's act
// names as Now.qml's lookActNames and the joined words Words.qml reads name
// them, the last of an id's winning, and the ids local-sites' home module
// guards as Now.qml lists them. A side saying nothing (no lapse, no link)
// agrees only where the other has nothing to say. The launcher's scripts
// run in Node, not in Qt's engine, which tests/for-you.qml holds them in;
// ask-reads-every-character.test.ts sweeps each character on its own. The
// check the-ask-row-reads-as-for-you hands it the launcher's files by the
// names `launcher` reads.

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { runInNewContext } from "node:vm";
import { type Ask, asked, LAUNCHER_ACTS, QUIET_AFTER } from "../kit/ask.ts";
import { CAME, DAY, EXPIRES, LOOKS } from "./ask-looks.ts";
import { launcher, loaded, type Script } from "./launcher.ts";

/** A look as Looks.js reads one, as far as this compares it. */
interface Read {
  readonly state: string;
  readonly words: string;
  readonly offers: readonly { value: string; label: string; follows: string }[];
}

/** For You as far as a page's row goes: Looks.js, LookForm.js, lookOffers and the act names. */
interface ForYou {
  readonly looks: Script;
  readonly form: Script;
  readonly offers: (one: Read) => Read["offers"];
  /** Whether a choice's drawer says what follows it, as Now.qml's followsOf decides. */
  readonly shows: (one: Read, choice: Read["offers"][number]) => boolean;
  /** The kinds Now.qml's drawer names a target for, which a page may link. */
  readonly linked: readonly string[];
  readonly acts: readonly string[];
  readonly open: string;
}

/** Now.qml's list of the launcher's acts on a look, by their words' ids. */
const ACT_IDS = /lookActNames:\s*\[([^\]]*)\]/u;
/** Now.qml's lookOffers, its body as written. */
const LOOK_OFFERS = /function lookOffers\(one\) \{\n([\s\S]*?)\n {4}\}/u;
/** What keeps Now.qml's followsOf from saying what follows a choice. */
const FOLLOWS_UNSAID = /const followsOf = choice => (.+?) \? \{\} : \{/u;
/** The kinds Now.qml's lookDrawer names a target for. */
const DRAWN_KINDS = /const target = (\[[^\]]*\])\.includes\(one\.kind\)/u;
const QUOTED = /"([^"]+)"/gu;
/** What For You calls opening a look, which its act is where it names none. */
const OPEN_ID = "verb:on:look:open";

/**
 * The launcher's words, each name by its id, as Words.qml reads the joined
 * words.json: its things in order, the last of an id's the one it names.
 */
function launcherWords(): Map<string, unknown> {
  const { things } = JSON.parse(launcher("WORDS")) as { things?: unknown };
  const names = new Map<string, unknown>();
  for (const thing of Array.isArray(things) ? things : []) {
    const { id, name } = (thing ?? {}) as { id?: unknown; name?: unknown };
    if (typeof id === "string") {
      names.set(id, name);
    }
  }
  return names;
}

/** A word's name, or what says it has none. */
function nameOf(names: Map<string, unknown>, id: string): string {
  const name = names.get(id);
  return typeof name === "string" ? name : `no ${id}`;
}

/** The launcher's acts on a look, by their words' ids, as Now.qml's lookActNames lists them. */
function actIds(): string[] {
  const ids = ACT_IDS.exec(launcher("NOW_QML"))?.[1];
  assert.ok(ids !== undefined, "Now.qml holds no lookActNames list to read");
  return Array.from(ids.matchAll(QUOTED), ([, id]) => id ?? "");
}

/** The launcher's act names on a look, as Now.qml's lookActNames names them. */
function actNames(): string[] {
  const names = launcherWords();
  return actIds().map((id) => nameOf(names, id));
}

/** A piece of Now.qml, as `pattern` finds it. */
function nowSays(pattern: RegExp, what: string): string {
  const found = pattern.exec(launcher("NOW_QML"))?.[1];
  assert.ok(found !== undefined, `Now.qml holds no ${what} to read`);
  return found;
}

/** The launcher's own scripts and words, loaded as For You loads them. */
function forYou(): ForYou {
  const order = loaded("ORDER");
  const form = loaded("LOOK_FORM");
  const looks = loaded("LOOKS", {
    ForYou: loaded("FOR_YOU", { Order: order }),
    LookForm: form,
    Order: order,
  });
  const acts = actNames();
  const offers = runInNewContext(
    `(function lookOffers(one) {${nowSays(LOOK_OFFERS, "lookOffers")}})`,
    { LookForm: form, lookActNames: acts },
  ) as ForYou["offers"];
  const shows = runInNewContext(
    `(function shows(one, choice) { return !(${nowSays(FOLLOWS_UNSAID, "followsOf")}); })`,
    { LookForm: form },
  ) as ForYou["shows"];
  const linked = JSON.parse(nowSays(DRAWN_KINDS, "lookDrawer's kinds")) as string[];
  return { looks, form, offers, shows, linked, acts, open: nameOf(launcherWords(), OPEN_ID) };
}

/** Moments either side of each that ends a wait: its expiry, and its quiet. */
const MOMENTS = [CAME, EXPIRES - 1, EXPIRES, CAME + QUIET_AFTER, CAME + QUIET_AFTER + 1];

/** A choice as both sides offer it: what it sends, what it's named and what follows it. */
const sent = ({ value, label, follows }: Read["offers"][number]): string =>
  `${value}: ${label}: ${follows.trim()}`;

/** A moment past every quiet and expiry the looks above say. */
const LONG_AFTER = CAME + QUIET_AFTER + DAY;

/** The schemes a page links, its own rule, since For You hands any target to xdg-open. */
const LINKS = ["http:", "https:"];

/** Where a page's act should take him: a kind For You's drawer names, at a web address. */
function linkFor(
  { kind, target }: { kind?: unknown; target?: unknown },
  linked: readonly string[],
): string | null {
  if (typeof kind !== "string" || typeof target !== "string" || !linked.includes(kind)) {
    return null;
  }
  const address = URL.canParse(target) ? new URL(target) : null;
  return address !== null && LINKS.includes(address.protocol) ? address.href : null;
}

/** Where the row and For You part, for one look at one moment: none where they agree. */
function parted(ask: Ask | null, read: Read | null, now: number, shell: ForYou): string[] {
  if (ask === null || read === null) {
    return ask === null && read === null ? [] : ["read whole otherwise"];
  }
  const { looks, form, offers, shows, linked, acts, open } = shell;
  const held = looks.quieted(read as never, now as never) as Read;
  const waits = held.state === "waiting" && looks.asksNothing(held as never) === false;
  // For You offers a waiting look's choices under To Read too, where the
  // row, stricter, offers none on a look not asking (ask.ts). It says what
  // follows each where Now.qml's own followsOf does.
  const offered = waits
    ? offers(held).map((one) => sent({ ...one, follows: shows(read, one) ? one.follows : "" }))
    : [];
  const opens = linkFor(read as never, linked);
  const act = (form.actOf(read as never, acts as never) as string) || open;
  const lapsed = (at: number): boolean =>
    (looks.quieted(read as never, at as never) as Read).state !== "waiting";
  // The first form is laid out apart on purpose, its question on its own.
  const second = form.secondForm(read as never) === true;
  const leads = !second || ask.said.join(" ") === read.words;
  // What For You's row asks and its drawer says follows doing nothing,
  // while the row, stricter, asks, each as For You reads it.
  const question = second ? "" : String(looks.askOf(read as never));
  const ifNothing = String(looks.ifNothingOf(held as never));
  return [
    leads ? "" : `leads with ${ask.said.join(" ")}, not ${read.words}`,
    !ask.asks || ask.question === question ? "" : `asks ${ask.question}, not ${question}`,
    !ask.asks || ask.ifNothing === ifNothing
      ? ""
      : `says ${ask.ifNothing} if nothing, not ${ifNothing}`,
    ask.asks === waits ? "" : "waits otherwise",
    ask.choices.map(sent).join(" | ") === offered.join(" | ") ? "" : "offers otherwise",
    ask.opens === opens ? "" : `opens ${ask.opens}, not ${opens}`,
    (ask.act === null) === (opens === null) ? "" : "links otherwise than it acts",
    ask.act === null || ask.act === act ? "" : `acts as ${ask.act}, not ${act}`,
    ask.lapses === null || (!lapsed(ask.lapses - 1) && lapsed(ask.lapses))
      ? ""
      : "lapses otherwise",
    !waits || ask.lapses !== null || !lapsed(LONG_AFTER) ? "" : "never lapses",
  ].filter((one) => one !== "");
}

it("reads, holds waiting, offers, acts and lapses as Looks.js, LookForm.js and lookOffers do", () => {
  const shell = forYou();
  for (const [name, look] of Object.entries(LOOKS)) {
    const text = JSON.stringify(look);
    for (const now of MOMENTS) {
      const read = shell.looks.look(text as never) as Read | null;
      const ask = asked(JSON.parse(text), now);
      assert.deepEqual(parted(ask, read, now, shell), [], `${name}, at ${now}`);
    }
  }
});

it("names the launcher's acts as Now.qml's lookActNames and its words do", () => {
  assert.deepEqual([...LAUNCHER_ACTS], actNames(), "the kit names the launcher's acts otherwise");
  assert.deepEqual(
    JSON.parse(launcher("LOOK_ACTS")),
    actIds(),
    "local-sites' home module guards other acts than Now.qml lists",
  );
});
