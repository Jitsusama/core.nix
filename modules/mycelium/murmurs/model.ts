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

/** How many characters of a murmur the lede quotes before it trims. */
const QUOTED = 140;

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
    murmurs.push({
      key: record.key,
      writer,
      quest: called(text(record, "purpose") ?? "", writer),
      words,
      ties: (ties ?? []).map(short).filter((each) => each !== writer),
      settled: settled === undefined || settled === "" ? null : settled,
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
    return { lead: "Nothing murmured", turn: "yet.", said, open };
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

/** The line under the sentence: the newest murmur still in the air, and who said it. */
function lede(murmurs: readonly Murmur[]): string {
  if (murmurs.length === 0) {
    return "The cells murmur here what they notice, and tie it to the quests it touches.";
  }
  const newest = inTheAir(murmurs)[0];
  if (newest === undefined) {
    return "Every murmur has been settled; the newest says how on its row.";
  }
  return `Newest in the air, from ${newest.quest}: ${trimmed(newest.words, QUOTED)}`;
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
export { ago, headline, inTheAir, lede, murmursOf, questsOf, short, trimmed };
