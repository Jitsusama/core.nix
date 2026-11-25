// The asks waiting on Joel, drawn on a page as ask rows (ask.css) and
// answered where they're asked: the to-do feed's looks, each read as For
// You reads it (ask.ts), the newest first, every one waiting or only those
// a cell left, by its address, as the map draws a cell's own.
//
// Every word in a row is free text a session wrote, so each is set as an
// element's text, never as markup: a title saying <b>Merge</b> shows those
// characters. A row offers its answers only while the feed is live, since a
// look held from a stale feed may have been answered since, and only until
// one is pressed, as For You does; what came of the press is said in the
// row, in a line a screen reader hears. A look that lapses while the page is
// open stops asking at that moment, by a timer ask.ts's waitFor sets.

import { type Ask, asked, type Choice, waitFor } from "./ask.ts";
import { carry, type Readying, ready, type Send } from "./answer.ts";
import { type Clock, follow, type Reaching, type State } from "./feed.ts";
import { type Drawn, type Holder, keyed } from "./keyed.ts";
import { type Held, isObject } from "./records.ts";
import { clockTime } from "./status.ts";

/** As much of an element as a row is built from, so a test can stand in. */
interface Made<Item> extends Drawn {
  textContent: string | null;
  className: string;
  append(...nodes: Item[]): void;
  addEventListener(type: "click", listener: () => void): void;
}

/** How a page makes an element: the document's createElement. */
type Make<Item> = (tag: string) => Item;

/** One look waiting on him, as its row draws it. */
interface Waiting {
  /** The feed's record key, which the row is kept by. */
  readonly key: string;
  /** The look, whole, read again at a press. */
  readonly look: unknown;
  readonly ask: Ask;
}

/** How a row stands beside its look: whether it offers answers, and what it says. */
interface View {
  readonly offering: boolean;
  /** What came of a press, or why no answers show; "" for nothing. */
  readonly told: string;
  readonly press: (choice: Choice) => void;
}

/** A day, in milliseconds: a lapse further off names its day. */
const DAY = 86_400_000;

const DAYS = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

/** When `look` came, for the order rows show in; 0 where it doesn't say. */
function cameAt(look: unknown): number {
  const at = isObject(look) ? look["at"] : undefined;
  if (typeof at === "number") {
    return Number.isFinite(at) ? at : 0;
  }
  const came = typeof at === "string" ? Date.parse(at) : Number.NaN;
  return Number.isNaN(came) ? 0 : came;
}

/** Whether `look` was left by the cell at `address`, read as the node reads one. */
function leftBy(look: unknown, address: string): boolean {
  const left = isObject(look) ? look["address"] : undefined;
  return typeof left === "string" && left.toUpperCase() === address.toUpperCase();
}

/**
 * The looks in `held` waiting on him at `now`, the newest first: every one,
 * or only those the cell at `address` left.
 */
function waiting(held: Held, now: number, address: string | null = null): readonly Waiting[] {
  const rows: { waiting: Waiting; at: number }[] = [];
  for (const record of held.all()) {
    const look = record["look"];
    const ask = asked(look, now);
    if (ask?.asks === true && (address === null || leftBy(look, address))) {
      rows.push({ waiting: { key: record.key, look, ask }, at: cameAt(look) });
    }
  }
  rows.sort((one, other) => other.at - one.at);
  return rows.map((row) => row.waiting);
}

/** What happens if he does nothing, in a sentence, as For You fills it from a lapse. */
function ifNothing(ask: Ask, now: number): string {
  if (ask.ifNothing !== "" || ask.lapses === null) {
    return ask.ifNothing;
  }
  const at = clockTime(ask.lapses);
  const day = ask.lapses - now < DAY ? "" : ` on ${DAYS[new Date(ask.lapses).getDay()]}`;
  return `It stops waiting${day} at ${at}.`;
}

/** An element of `tag` made by `make`, of `kind` and saying `words`, set as text. */
function said<Item extends Made<Item>>(
  make: Make<Item>,
  tag: string,
  words: string,
  kind = "",
): Item {
  const element = make(tag);
  element.className = kind;
  element.textContent = words;
  return element;
}

/** `words`, a link to `opens` where it's a web address, as text otherwise. */
function linked<Item extends Made<Item>>(
  make: Make<Item>,
  words: string,
  opens: string | null,
): Item {
  if (opens === null) {
    return said(make, "span", words);
  }
  const link = said(make, "a", words);
  link.setAttribute("href", opens);
  return link;
}

/** The row's answers: a button for each choice, beside what follows from it. */
function answers<Item extends Made<Item>>(make: Make<Item>, ask: Ask, view: View): Item {
  const list = said(make, "ul", "", "ask-answers");
  for (const choice of ask.choices) {
    const button = said(make, "button", choice.label);
    button.setAttribute("type", "button");
    button.addEventListener("click", () => view.press(choice));
    const item = make("li");
    item.append(button);
    if (choice.follows !== "") {
      item.append(said(make, "span", choice.follows));
    }
    list.append(item);
  }
  return list;
}

/** One look's row, as ask.css lays it out, every word set as text. */
function drawAsk<Item extends Made<Item>>(
  make: Make<Item>,
  row: Waiting,
  view: View,
  now: number,
): Item {
  const { ask } = row;
  const article = said(make, "article", "", "ask");
  const title = make("h3");
  title.append(linked(make, ask.title, ask.opens));
  article.append(title);
  for (const words of ask.said) {
    article.append(said(make, "p", words, "ask-said"));
  }
  const question = [ask.question, ifNothing(ask, now)].filter((one) => one !== "").join(" ");
  if (question !== "") {
    article.append(said(make, "p", question, "ask-question"));
  }
  if (view.offering) {
    article.append(answers(make, ask, view));
  }
  // Always there, so what a press comes to is heard when it's written.
  const told = said(make, "p", view.told, "ask-told");
  told.setAttribute("aria-live", "polite");
  article.append(told);
  const act = make("p");
  act.className = "ask-act";
  act.append(ask.act === null ? said(make, "span", ask.target) : linked(make, ask.act, ask.opens));
  article.append(act);
  return article;
}

/** Why a row with nothing pressed offers no answers, or what more it takes. */
function unoffered(ask: Ask, live: boolean): string {
  if (ask.choices.length === 0) {
    return ask.takesWords ? "It asks for your own words, which you give in For You." : "";
  }
  if (!live) {
    return "Its answers come back once this page is live again; For You answers it meanwhile.";
  }
  return ask.takesWords ? "Or answer in your own words in For You." : "";
}

/** What a page hands `followAsks`. */
interface Asking<Item extends Drawn> {
  /** Where the rows are drawn; the list owns every child of it. */
  readonly holder: Holder<Item>;
  readonly make: Make<Item>;
  /** Only the asks of the cell at this address, until narrowed again; every one without it. */
  readonly address?: string;
  /** Told how many rows show after each drawing, so a page can hide an empty part. */
  readonly counted?: (count: number) => void;
  readonly reaching?: Reaching & { readonly send?: Send; readonly ready?: Readying };
}

const browserClock: Clock = {
  now: () => Date.now(),
  later: (milliseconds, then) => {
    const timer = setTimeout(then, milliseconds);
    return () => clearTimeout(timer);
  },
};

/** The asks a page follows: narrowed to one cell's or widened to all, and closed. */
interface Following {
  /** Shows only the asks of the cell at `address`, or every one for null. */
  readonly narrow: (address: string | null) => void;
  readonly close: () => void;
}

/** Follows the to-do feed's looks into `asking`'s rows, until closed. */
function followAsks<Item extends Made<Item>>(asking: Asking<Item>): Following {
  const reaching = asking.reaching ?? {};
  let address = asking.address ?? null;
  const clock = reaching.clock ?? browserClock;
  const pressed = new Map<string, string>();
  let state: State = "connecting";
  let unwait: (() => void) | null = null;
  let held: Held | null = null;
  // Whether the server takes this page's answers: unknown until it says,
  // then null where it does, or the sentence saying why it doesn't.
  let barred: string | null | undefined;
  const press = (row: Waiting, choice: Choice): void => {
    if (pressed.has(row.key)) {
      return;
    }
    pressed.set(row.key, "Sending your answer.");
    draw([row.key]);
    carry(row.look, choice, clock.now(), reaching.send).then((told) => {
      pressed.set(row.key, told.words);
      draw([row.key]);
    });
  };
  const viewOf = (row: Waiting): View => {
    const told = pressed.get(row.key);
    const live = state === "live";
    const offers = row.ask.choices.length > 0;
    const refused = offers && typeof barred === "string" ? barred : null;
    return {
      offering: told === undefined && live && offers && barred === null,
      told: told ?? refused ?? unoffered(row.ask, live),
      press: (choice) => press(row, choice),
    };
  };
  function draw(moved?: Iterable<string>): void {
    if (held === null) {
      return;
    }
    const now = clock.now();
    const rows = waiting(held, now, address);
    keyed(asking.holder, {
      rows,
      key: (row) => row.key,
      draw: (row) => drawAsk(asking.make, row, viewOf(row), now),
      ...(moved === undefined ? {} : { moved }),
    });
    asking.counted?.(rows.length);
    unwait?.();
    const waits = rows
      .map((row) => waitFor(row.ask, now))
      .filter((wait): wait is number => wait !== null);
    unwait = waits.length === 0 ? null : clock.later(Math.min(...waits), () => draw());
  }
  const following = follow(
    { feed: "to-do", kind: "look" },
    {
      moved: (_, moved) => draw(moved.whole ? undefined : [...moved.changed, ...moved.gone]),
      stands: (standing) => {
        state = standing.state;
        draw();
      },
      // A record the page can't read is one look it can't show; For You still holds it.
      problem: () => undefined,
    },
    reaching,
  );
  held = following.held;
  ready(reaching.ready).then((why) => {
    barred = why;
    draw();
  });
  return {
    narrow: (next) => {
      if (next !== address) {
        address = next;
        draw();
      }
    },
    close: () => {
      unwait?.();
      following.close();
    },
  };
}

export type { Asking, Following, Made, Make, View, Waiting };
export { drawAsk, followAsks, waiting };
