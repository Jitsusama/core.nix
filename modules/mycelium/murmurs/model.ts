// What the murmurs page draws, read from the mycelium node's murmur records
// (mycelium's PROTOCOL.md): one for each murmur this machine's cells said
// into the quests' webs over the last day, keyed murmur:<cell>:<at>, with
// the cell that said it, its words, the quests it is tied to, and how it
// was settled once it is. Nothing here touches the page, so every case is
// proven in node, and a record the page couldn't make sense of is said
// rather than drawn as if it were whole.

import type { Held } from "../../local-sites/kit/records.ts";
import { number, text, texts } from "../../local-sites/kit/records.ts";

/** One murmur, as the page draws its bird and its row. */
interface Murmur {
  readonly key: string;
  /**
   * The cell that said it, by its address, such as KESTRL. A quest's cell
   * goes by its quest's short id, which is how a tie finds its flock.
   */
  readonly writer: string;
  /** What its cell works on: its purpose up to the first colon. */
  readonly quest: string;
  readonly words: string;
  /** The quests it is tied to, by their short ids, its own cell's left out. */
  readonly ties: readonly string[];
  /** How it was settled, or null while it's still in the air. */
  readonly settled: string | null;
  /** When it was said, in seconds since 1970. */
  readonly at: number;
}

/** Every murmur held, newest first, and what couldn't be read. */
interface Murmurs {
  readonly murmurs: readonly Murmur[];
  readonly unread: readonly string[];
}

/** The page's sentence for the whole, as a lead and its turn. */
interface Headline {
  readonly lead: string;
  readonly turn: string;
  readonly said: number;
  readonly open: number;
}

/** The words of numbers up to ten, as a sentence opens with them. */
const WORDS: readonly string[] = [
  "No",
  "One",
  "Two",
  "Three",
  "Four",
  "Five",
  "Six",
  "Seven",
  "Eight",
  "Nine",
  "Ten",
];

/** A stop that would sit before the ellipsis a cut murmur ends with. */
const TRAILING_STOP = /[,;:.]$/u;
/**
 * The controls that embed, override or isolate a direction, which could close the isolates the
 * page holds a writer's words in, or run on past them, and the separators that end a paragraph
 * for direction, and every isolate open in it, said as the space they stand for.
 */
const DIRECTIONS = /[\u202A-\u202E\u2066-\u2069]/gu;
const PARAGRAPHS: ReadonlySet<string> = new Set(["\u001C", "\u001D", "\u001E", "\u0085", "\u2029"]);

/**
 * How many characters of a murmur the lede quotes, and of its quest's name, before each is
 * trimmed: the most that keeps the whole lede inside the four lines its box holds on a 340 px
 * phone, so a 390 px one, the width the sites are checked at, holds it in any browser, and
 * nothing in it is ever cut away. Browsers set the same words a little apart: cut at 84 and
 * 48, the lede that fit four lines at 390 in one took five in another. The murmur's row says
 * both whole. It's sized for prose: wide letters in both, a word too long to break or a
 * narrower phone takes a fifth line, which the box grows to hold rather than cutting it,
 * moving what's under it down a line as the lede draws.
 */
const QUOTED = 76;
const NAMED = 40;

/** A quest's short id, from its whole id or the short one itself. */
function short(quest: string): string {
  return quest.slice(quest.lastIndexOf("-") + 1);
}

/** What a quest is called: its purpose up to the first colon, or its id. */
function called(purpose: string, writer: string): string {
  const name = purpose.split(":")[0]?.trim() ?? "";
  return name === "" ? writer : name;
}

/** `count` as a sentence opens with it: a word to ten, a numeral past. */
function counted(count: number): string {
  return WORDS[count] ?? String(count);
}

/** `words` as a writer's text is drawn: without the controls of direction it held. */
function plain(words: string): string {
  const said = [...words.replace(DIRECTIONS, "")];
  return said.map((character) => (PARAGRAPHS.has(character) ? " " : character)).join("");
}

/** Every murmur `held` holds, newest first, with each it couldn't read named. */
function murmursOf(held: Held): Murmurs {
  const murmurs: Murmur[] = [];
  const unread: string[] = [];
  for (const record of held.all()) {
    const writer = text(record, "writer");
    const words = text(record, "text");
    const at = number(record, "at");
    if (writer === undefined || words === undefined || at === undefined) {
      unread.push(`${record.key}, which names no writer, words or time`);
      continue;
    }
    // A tie list or a settling that isn't what the node says it sends is
    // drawn as none, and said, so a murmur is never shown settled or
    // untied by a misreading nobody hears of.
    const { tied_to: tiedTo = [], settled: said = null } = record;
    const ties = texts(record, "tied_to");
    if (ties === undefined && tiedTo !== null) {
      unread.push(`${record.key}'s ties, which aren't a list of names`);
    }
    const settled = text(record, "settled");
    if (settled === undefined && said !== null) {
      unread.push(`${record.key}'s settling, which isn't words`);
    }
    // Plain before it's judged, so a settling of nothing but controls is as empty as "".
    const how = plain(settled ?? "");
    const who = plain(writer);
    murmurs.push({
      key: record.key,
      writer: who,
      quest: called(plain(text(record, "purpose") ?? ""), who),
      words: plain(words),
      ties: (ties ?? []).map((each) => plain(short(each))).filter((each) => each !== who),
      settled: how === "" ? null : how,
      at,
    });
  }
  murmurs.sort((one, other) => other.at - one.at || one.key.localeCompare(other.key));
  return { murmurs, unread };
}

/** The murmurs still in the air, newest first. */
function inTheAir(murmurs: readonly Murmur[]): readonly Murmur[] {
  return murmurs.filter((each) => each.settled === null);
}

/** The page's sentence: how many were murmured, and how many are still in the air. */
function headline(murmurs: readonly Murmur[]): Headline {
  const open = inTheAir(murmurs).length;
  const said = murmurs.length;
  if (said === 0) {
    // As long as a count's, so the head keeps the lines it opens on.
    return { lead: "Nothing murmured,", turn: "nothing in the air.", said, open };
  }
  const lead = `${counted(said)} murmured,`;
  if (open === 0) {
    // "all", as "all still in the air." says, since "every one" took a phone's headline a line
    // past every other count and the words the page opens with.
    return { lead, turn: "all come to roost.", said, open };
  }
  if (open === said) {
    return { lead, turn: "all still in the air.", said, open };
  }
  return { lead, turn: `${counted(open).toLowerCase()} still in the air.`, said, open };
}

/** `words` cut at `most` characters on a word, with an ellipsis of three stops. */
function trimmed(words: string, most: number): string {
  // Counted in characters as a reader sees them, so a cut never splits one.
  const characters = [...words];
  if (characters.length <= most) {
    return words;
  }
  // One character past the most, so a word ending just there is kept whole.
  const cut = characters.slice(0, most + 1).join("");
  const space = cut.lastIndexOf(" ");
  const kept = space > 0 ? cut.slice(0, space) : characters.slice(0, most).join("");
  return `${kept.replace(TRAILING_STOP, "")}...`;
}

/** What the page says while it hasn't heard the node, beside the words it opens on. */
const UNHEARD = {
  headline: "Not hearing the mycelium node.",
  lede: "The mycelium node didn't say what its sessions murmur; the line below says why.",
};

/**
 * `words` cut at `most`, between a first-strong isolate and its pop, the plain text form of
 * `<bdi>`, so a direction they set, or one a cut leaves open, runs no further than them. The
 * words are plain, as murmursOf reads them, so none of theirs can close the isolate early.
 */
function isolated(words: string, most: number): string {
  return `\u2068${trimmed(words, most)}\u2069`;
}

/** The line under the sentence: the newest murmur still in the air, and who said it. */
function lede(murmurs: readonly Murmur[]): string {
  if (murmurs.length === 0) {
    return "The cells murmur here what they notice, and tie it to the quests it touches.";
  }
  const newest = inTheAir(murmurs)[0];
  if (newest === undefined) {
    return "Every murmur has been settled; the newest says how on its row.";
  }
  const quest = isolated(newest.quest, NAMED);
  return `Newest in the air, from ${quest}: ${isolated(newest.words, QUOTED)}`;
}

/** How long ago `at` was, from `now`, both in seconds, said short. */
function ago(at: number, now: number): string {
  const seconds = Math.max(0, now - at);
  if (seconds < 60) {
    return "just now";
  }
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) {
    return `${minutes} min ago`;
  }
  const hours = Math.floor(minutes / 60);
  if (hours < 48) {
    return `${hours} h ago`;
  }
  return `${Math.floor(hours / 24)} days ago`;
}

/** The quests murmuring, each once, in the order their newest murmur was said. */
function questsOf(murmurs: readonly Murmur[]): readonly string[] {
  return [...new Set(murmurs.map((each) => each.writer))];
}

export type { Headline, Murmur, Murmurs };
export { ago, headline, inTheAir, lede, murmursOf, questsOf, short, trimmed, UNHEARD };
