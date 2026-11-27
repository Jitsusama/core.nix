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
 * Whether the first of `read`'s letters, as many as `each` has, by code
 * point, may draw as `each`: each the act's own letter, or one not LATIN
 * where the act has a letter. A space draws only as a space.
 */
function drawsLike(read: string, each: string): boolean {
  const ours = Array.from(read);
  const acts = Array.from(each);
  return (
    ours.length >= acts.length &&
    acts.every(
      (one, at) =>
        ours[at] === one || (one !== " " && ours[at] !== " " && !LATIN.test(ours[at] ?? " ")),
    )
  );
}

/** How many code points `text` holds, as drawsLike counts them. */
function pointsIn(text: string): number {
  return Array.from(text).length;
}

export { drawsLike, pointsIn };
