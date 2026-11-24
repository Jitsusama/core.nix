// What a cell's drawer offers Joel, in the order he reaches for it, and what
// it asks him, said whole. A cell that needs him is one he must get to, so
// Go to It comes first: the link the desktop hands the launcher, which raises
// the cell's own window, or opens one where it has none, as a notice's Go To
// does (modules/quickshell/mycelium-link). Then talking to it, starting new
// work from it, and reading what it is for. What it asks is the whole text its
// session ended on where the session says it, never only its last line, which
// cut a question to its closing words ("2. The signing pause: yes, no or
// partly?") and left him nothing to answer by.

import { cellLink } from "../../local-sites/kit/cell.ts";
import { isAddress } from "./sprout.ts";

/** One thing the drawer offers, by what it does, the words it is said in and where it goes. */
export type Act =
  | { readonly act: "go"; readonly words: string; readonly href: string }
  | { readonly act: "talk"; readonly words: string; readonly href: string }
  | { readonly act: "sprout"; readonly words: string; readonly from: string }
  | { readonly act: "forage"; readonly words: string; readonly href: string };

/** A cell as its drawer's acts read it. */
export interface Acting {
  readonly address: string;
  /** The machine it runs on, never the basin it is drawn from. */
  readonly host?: string | null | undefined;
  /** Whether it is a receptor, which only senses and is never talked to. */
  readonly receptor: boolean;
  /** Whether it has a forage, which the forage page lists. */
  readonly foraged: boolean;
}

/** The words of each act, said the same in every drawer. */
export const ACT_WORDS = {
  go: "Go to It",
  talk: "Talk to It in the Launcher",
  sprout: "Start New Work From It",
  forage: "See What It Is For, and Its Plans",
} as const;

/**
 * The link that goes to the cell at `address`: its window raised, or one
 * opened, saying the machine it runs on where a link may carry it. None
 * where the address isn't one the launcher could reach.
 */
export const goLink = (address: string, host?: string | null): string | null => {
  const link = cellLink(address, host);
  return link === null ? null : `${link}/go`;
};

/**
 * What the drawer offers, first to last: Go to It, Talk to It and Start New
 * Work for a session whose address a link may carry, and What It Is For where
 * it has a forage. A receptor offers only its forage, since there is no
 * window to go to and nothing to talk with.
 */
export const cellActs = (s: Acting): readonly Act[] => {
  const acts: Act[] = [];
  const talk = cellLink(s.address, s.host);
  const go = goLink(s.address, s.host);
  if (!s.receptor && go !== null && talk !== null) {
    acts.push({ act: "go", words: ACT_WORDS.go, href: go });
    acts.push({ act: "talk", words: ACT_WORDS.talk, href: talk });
  }
  if (!s.receptor && isAddress(s.address)) {
    acts.push({ act: "sprout", words: ACT_WORDS.sprout, from: s.address });
  }
  if (s.foraged) {
    acts.push({
      act: "forage",
      words: ACT_WORDS.forage,
      href: `../forage/#cell=${encodeURIComponent(s.address)}`,
    });
  }
  return acts;
};

/** What a cell asks, as the node says it. */
export interface Asking {
  /** Its one short line, as every one-line reader shows it. */
  readonly wants?: string | null | undefined;
  /** The whole text it asked in, where its session says it. */
  readonly asked?: string | null | undefined;
}

// A blank line, which parts one paragraph from the next.
const PARAGRAPHS = /\n[^\S\n]*\n\s*/u;

/**
 * What the cell asks, whole: its paragraphs in order, each its lines, so a
 * list it asked in reads as the list it is. The whole text where its session
 * says it, else its short line, and none where it asks nothing.
 */
export const askedOf = (s: Asking): readonly (readonly string[])[] => {
  const whole = (s.asked ?? "").replace(/\r\n?/gu, "\n").trim();
  const said = whole === "" ? (s.wants ?? "").trim() : whole;
  return said
    .split(PARAGRAPHS)
    .map((paragraph) =>
      paragraph
        .split("\n")
        .map((line) => line.trim())
        .filter((line) => line !== ""),
    )
    .filter((lines) => lines.length > 0);
};
