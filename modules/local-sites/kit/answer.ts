// An answer Joel pressed on one of the sites' own pages, posted to the
// sites' server, which checks it was pressed here and carries it to the
// session that asked as the launcher's For You does (src/answer.rs), and
// what the server said, in a sentence for the row to say.
//
// The page reads the look again at the press (ask.ts `answering`), so an
// honest page never posts what no longer asks; the server reads it again
// as well, since it can't trust what any page posts. Each refusal is said
// plainly with what to do next, and every one that the page can't mend
// sends him to For You, which answers every look in place.
//
// Before it offers an answer, a page asks the server once (`ready`): the
// server then gives it what its answers carry, only where it can tell this
// browser is Joel's own and not one of his agents'.

import { answering, type Choice } from "./ask.ts";
import { isObject } from "./records.ts";

/** What the server answered a post: its status and its body. */
interface Posted {
  readonly status: number;
  readonly text: string;
}

/** How an answer's body is posted, so a test can stand in for the server. */
type Send = (body: string) => Promise<Posted>;

/** How a page asks the server whether it takes its answers. */
type Readying = () => Promise<Posted>;

/** What came of a press, for the row to say. */
interface Told {
  /** Whether the server carried it to the session that asked. */
  readonly carried: boolean;
  readonly words: string;
}

/** The status the server answers a carried answer with. */
const CARRIED = 200;

/** The status the server gives a page the cookie its answers carry with. */
const GIVEN = 204;

/** The status of a server that takes no answers, on a machine in no realm. */
const TAKES_NONE = 404;

/** Where the sites' server takes an answer, on every page's own origin. */
const ANSWER = "/answer";

const FOR_YOU = "answer in For You";
const NOT_SENT = "so nothing was sent";
const NO_LONGER = `It no longer waits on you, ${NOT_SENT}.`;
const NOT_OURS = `Only a page of your sites can answer, ${NOT_SENT}: ${FOR_YOU}.`;
const UNREACHED = `It couldn't reach the session that asked: ${FOR_YOU}.`;
const TAKES_NO_ANSWERS = `This machine's sites take no answers: ${FOR_YOU}.`;
const UNSEEN = `The sites' server can't be reached, ${NOT_SENT}.`;

/** What each word the server refuses with says to him. */
const REFUSALS: Readonly<Record<string, string>> = {
  type: `This page sent its answer in a way the server doesn't take: ${FOR_YOU}.`,
  agent: `This browser runs as one of your agents, so it can't answer for you: ${FOR_YOU}.`,
  untold: `The server couldn't tell this press was yours: ${FOR_YOU}.`,
  origin: NOT_OURS,
  site: NOT_OURS,
  cookie: "The sites' server restarted since this page opened: reload it to answer.",
  unreadable: `The server couldn't read the answer this page sent: ${FOR_YOU}.`,
  answered: "It was answered already.",
  answering: "Your answer to it is already on its way.",
  unknown: `Its session has cleared it, ${NOT_SENT}.`,
  unwhole: `It can't be read whole, ${NOT_SENT}: ${FOR_YOU}.`,
  ended: NO_LONGER,
  unasked: NO_LONGER,
  lapsed: NO_LONGER,
  quiet: NO_LONGER,
  unoffered: `That answer is no longer offered, ${NOT_SENT}.`,
  unaddressed: `It doesn't say which session asked: ${FOR_YOU}.`,
  unread: `The server couldn't read it as it stands, ${NOT_SENT}: ${FOR_YOU}.`,
  unreached: UNREACHED,
  unsteered: UNREACHED,
};

/** The browser's post of `body` to the server that served the page. */
async function browserSend(body: string): Promise<Posted> {
  const answer = await fetch(ANSWER, {
    method: "POST",
    credentials: "same-origin",
    headers: { "Content-Type": "application/json" },
    body,
  });
  return { status: answer.status, text: await answer.text() };
}

/** The browser's ask of the server that served the page, for its cookie. */
async function browserAsk(): Promise<Posted> {
  const answer = await fetch(ANSWER, { credentials: "same-origin" });
  return { status: answer.status, text: await answer.text() };
}

/** What the server's refusal `posted` says to him. */
function refusedAs(posted: Posted): string {
  let said: unknown = null;
  try {
    said = JSON.parse(posted.text);
  } catch {
    // A server that takes no answers says so in plain words, not JSON.
  }
  const word = isObject(said) ? said["refused"] : undefined;
  const words = typeof word === "string" ? REFUSALS[word] : undefined;
  if (words !== undefined) {
    return words;
  }
  return posted.status === TAKES_NONE
    ? TAKES_NO_ANSWERS
    : `It wasn't sent (the server answered ${posted.status}): ${FOR_YOU}.`;
}

/**
 * Asks the server whether it takes this page's answers, and is given what
 * they carry if so: null where it does, else the sentence saying why not.
 */
async function ready(ask: Readying = browserAsk): Promise<string | null> {
  let asked: Posted;
  try {
    asked = await ask();
  } catch {
    return UNSEEN;
  }
  return asked.status === GIVEN ? null : refusedAs(asked);
}

/**
 * Posts `choice` as the answer to `look`, read again at `now`, and says
 * what came of it. Nothing is posted for a look that no longer asks it.
 */
async function carry(
  look: unknown,
  choice: Choice,
  now: number,
  send: Send = browserSend,
): Promise<Told> {
  const answer = answering(look, choice.value, now);
  if (answer === null) {
    return { carried: false, words: NO_LONGER };
  }
  let posted: Posted;
  try {
    posted = await send(JSON.stringify({ look: answer.look, value: answer.value }));
  } catch {
    return { carried: false, words: UNSEEN };
  }
  if (posted.status === CARRIED) {
    return { carried: true, words: `Answered ${choice.label}: sent to the session that asked.` };
  }
  return { carried: false, words: refusedAs(posted) };
}

export type { Posted, Readying, Send, Told };
export { carry, ready };
