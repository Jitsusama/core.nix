// Whether a name may draw as an act, letter by letter, where it may borrow
// a letter from beyond Latin that draws as one: "Clear It" with a Cyrillic
// Es (round 12 on #6562, F170), Lisu's capitals, a Roman numeral or a
// mathematical letter. A list of the letters that draw alike never ends,
// as three rounds on #6562 found (F170, then round 13's 177 to 187), so any
// letter beyond Latin is read as able to be any. And any may draw as
// nothing, as a mark combining with the letter before it does where MARKS
// doesn't name it, "Ta\u0485lk" drawing as Talk (#6562, 211), whose list
// never ends either, so each is tried both ways. The kit's ask row reads
// names through it, as LookForm.js's endsOf does for For You.

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
 * How many of `acts`' letters may be drawn once `one` is read after any of
 * `drawn`, as LookForm.js's drawnPast: one more where it draws as the act's
 * next letter, as its own, as one STEMS reads as it, or as one not LATIN
 * where the act has a letter; and as many where it's not LATIN, which may
 * draw as nothing. A space draws only as a space.
 */
function drawnPast(drawn: readonly number[], one: string, acts: readonly string[]): number[] {
  const loose = one !== " " && !LATIN.test(one);
  const next: number[] = [];
  for (const count of drawn) {
    const act = acts[count];
    const drawsAs = act !== undefined && (stem(one) === stem(act) || (loose && act !== " "));
    if (drawsAs && !next.includes(count + 1)) {
      next.push(count + 1);
    }
    if (loose && !next.includes(count)) {
      next.push(count);
    }
  }
  return next;
}

/**
 * Where in `read`, by code point, a drawing of `each` whole may end, in
 * order, as LookForm.js's endsOf: each letter not LATIN tried both as one of
 * the act's and as nothing (drawnPast), so "ta\u0485lk" draws as talk.
 */
function endsOf(read: string, each: string): number[] {
  const ours = Array.from(read);
  const acts = Array.from(each);
  const ends: number[] = [];
  let drawn = [0];
  for (let at = 0; drawn.length > 0; at += 1) {
    if (drawn.includes(acts.length)) {
      ends.push(at);
    }
    const one = ours[at];
    drawn = one === undefined ? [] : drawnPast(drawn, one, acts);
  }
  return ends;
}

/** How many code points `text` holds, as endsOf counts them. */
function pointsIn(text: string): number {
  return Array.from(text).length;
}

export { endsOf, MARKS, pointsIn };
