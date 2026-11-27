// Whether a name may draw as an act, letter by letter, where it may borrow
// a letter from beyond Latin that draws as one: "Clear It" with a Cyrillic
// Es (round 12 on #6562, F170), Lisu's capitals, a Roman numeral or a
// mathematical letter. A list of the letters that draw alike never ends,
// as three rounds on #6562 found (F170, then round 13's 177 to 187), so any
// letter beyond Latin is read as able to be any. The kit's ask row reads
// names through it, as LookForm.js's drawsLike does for For You.

/** The letters an act's are told apart from: ASCII's, and Latin's accented ones. */
const LATIN = /[\x21-\x7e\u00c0-\u017f]/u;

/**
 * Latin's own letters that draw alike in a plain face, each read as the one
 * it draws like, as LookForm.js's STEMS: a capital I, a small l, a dotless
 * i, a one and a bar as a stem, a zero as an o (round 14 on #6562, 198, 207).
 */
const STEMS: Record<string, string> = { i: "l", "1": "l", "|": "l", "\u0131": "l", "0": "o" };

/**
 * The marks that combine with the letter before them, drawn on it and taking
 * no place, read as nothing, as LookForm.js's MARKS (round 14, 201).
 */
const MARKS = /[\u0300-\u036f\u1ab0-\u1aff\u1dc0-\u1dff\u20d0-\u20ff\ufe20-\ufe2f]/gu;

/** The letter `one` is read as, as STEMS gives it. */
function stem(one: string | undefined): string | undefined {
  return one === undefined ? one : (STEMS[one] ?? one);
}

/**
 * Whether the first of `read`'s letters, as many as `each` has, by code
 * point, may draw as `each`: each the act's own letter, or one not LATIN
 * where the act has a letter, or one STEMS reads as the act's. A space
 * draws only as a space.
 */
function drawsLike(read: string, each: string): boolean {
  const ours = Array.from(read);
  const acts = Array.from(each);
  return (
    ours.length >= acts.length &&
    acts.every(
      (one, at) =>
        stem(ours[at]) === stem(one) ||
        (one !== " " && ours[at] !== " " && !LATIN.test(ours[at] ?? " ")),
    )
  );
}

/** How many code points `text` holds, as drawsLike counts them. */
function pointsIn(text: string): number {
  return Array.from(text).length;
}

export { drawsLike, MARKS, pointsIn };
