// What the repos daemon's sources couldn't read, as the words at the page's foot: each reason
// once, with every key it holds for, so a dozen repositories GitHub can't see read as one
// sentence rather than a dozen, and a key never repeats the name of the source it came from.

import { type Fields, isObject, text, texts } from "../../local-sites/kit/records.ts";

// A list said as words, its last two joined by "and" with no comma before it.
const SPOKEN = new Intl.ListFormat("en-GB", { type: "conjunction" });

/**
 * What one source couldn't read, each reason said once with every key it holds for, as "a and b
 * on github, not found": a source says each key as `key: reason`, and its keys may begin with
 * its own name, which the sentence says once rather than before each.
 */
function unreadBy(name: string, unread: readonly string[]): readonly string[] {
  const keys = new Map<string, string[]>();
  for (const each of unread) {
    const split = each.indexOf(": ");
    const whole = split < 0 ? each : each.slice(0, split);
    const key = whole.startsWith(`${name}:`) ? whole.slice(name.length + 1) : whole;
    const why = split < 0 ? "" : each.slice(split + 2);
    const held = keys.get(why);
    if (held === undefined) {
      keys.set(why, [key]);
    } else {
      held.push(key);
    }
  }
  return [...keys].map(
    ([why, named]) => `${SPOKEN.format(named)} on ${name}${why === "" ? "" : `, ${why}`}`,
  );
}

/** What the reading's sources couldn't read, each a sentence's part. */
function unreadOf(reading: Fields): readonly string[] {
  const told = reading["sources"];
  const sources = Array.isArray(told) ? told : [];
  return sources.flatMap((source: unknown) => {
    const unread = isObject(source) ? (texts(source, "unread") ?? []) : [];
    return isObject(source) ? unreadBy(text(source, "name") ?? "a source", unread) : [];
  });
}

export { unreadOf };
