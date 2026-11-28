// The closed alphabet a look's choice is trusted in, as LookForm.js holds it
// for For You, so a choice can't borrow a character that draws as an act's
// letter or as nothing: a Cyrillic Es as C (round 12 on #6562, F170), a kra
// as k, a times sign as x (218 and 224), a mark no list names over a letter
// (211). A list of those never ends, as round after round on #6562 found,
// so none is kept: a choice holding, once its accents are gone, anything
// outside the set poses as any act. The set's own letters that draw alike
// are read as one, and each of its characters is weighed as it draws. The
// kit's ask row reads names through it, as LookForm.js's poses does.

/**
 * The marks that combine with the letter before them, drawn on it and taking
 * no place, read as nothing, as LookForm.js's MARKS (round 14, 201).
 */
const MARKS = /[\u0300-\u036f\u1ab0-\u1aff\u1dc0-\u1dff\u20d0-\u20ff\ufe20-\ufe2f]/gu;

/**
 * The accents a choice's decomposed letters carry, as LookForm.js's
 * COMBINING, so "Ta\u013ck" decomposed reads as "Tal\u0327k" (#6562, 215).
 */
const COMBINING = /[\u0300-\u036f]/gu;

/**
 * The only characters a choice is trusted to hold, each with how wide it
 * draws in small letters, as LookForm.js's WIDTHS: a capital or a digit half
 * as wide again, and an m, a w and the marks drawn as wide as one about
 * twice, so neither $ nor _ is in it to be weighed as a small letter (214).
 */
const WIDTHS = widthsOf([
  ["abcdefghijklnopqrstuvxyz .,'\"-!?:;()/\u2018\u2019\u201c\u201d\u2013", 1],
  ["ABCDEFGHIJKLNOPQRSTUVXYZ0123456789", 1.5],
  ["mwMW&#+\u2014", 2],
]);

/**
 * The set's letters that draw alike in a plain face, each read as the one it
 * draws like, as LookForm.js's ALIKE: a stem as an l, a zero as an o, two v's
 * as a w, r and n as an m, and c and l as a d (round 14, 198 and 207; 218).
 */
const ALIKE: readonly (readonly [RegExp, string])[] = [
  [/[i1|]/gu, "l"],
  [/0/gu, "o"],
  [/vv/gu, "w"],
  [/rn/gu, "m"],
  [/cl/gu, "d"],
];

/** What goes on a word, as LookForm.js's WORDED: a letter, or a digit. */
const WORDED = /[a-z0-9]/u;

/** The table of WIDTHS from its `groups`, as LookForm.js's widthsOf. */
function widthsOf(groups: readonly (readonly [string, number])[]): ReadonlyMap<string, number> {
  const table = new Map<string, number>();
  for (const [characters, width] of groups) {
    for (const one of characters.split("")) {
      table.set(one, width);
    }
  }
  return table;
}

/** Whether every character of `bare` is in the closed set. */
function trusted(bare: string): boolean {
  return bare.split("").every((one) => WIDTHS.has(one));
}

/**
 * How wide one character draws, as LookForm.js's widthOf: as WIDTHS says, and
 * one outside the set as the widest.
 */
function widthOf(each: string): number {
  return WIDTHS.get(each) ?? 2;
}

/** `read` as it draws in a plain face, as LookForm.js's skeletonOf. */
function skeletonOf(read: string): string {
  return ALIKE.reduce((drawn, [alike, as]) => drawn.replace(alike, as), read.toLowerCase());
}

/** Whether `read` draws as `each` whole, as LookForm.js's reads. */
function reads(read: string, each: string): boolean {
  return skeletonOf(read) === skeletonOf(each);
}

/**
 * Whether `read` opens with the words `each` and then a break between words,
 * as LookForm.js's opensWith: "clear it, now" opens with "clear it", where
 * "clearance" doesn't open with "clear"; by any drawing of it (skeletonOf),
 * so "ansvver later" opens with answer.
 */
function opensWith(read: string, each: string): boolean {
  const act = skeletonOf(each);
  for (let end = 1; end < read.length; end += 1) {
    if (!WORDED.test(read.charAt(end)) && skeletonOf(read.slice(0, end)) === act) {
      return true;
    }
  }
  return false;
}

export { COMBINING, MARKS, opensWith, reads, trusted, widthOf };
