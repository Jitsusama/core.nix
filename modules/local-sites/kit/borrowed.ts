// A letter another script lends that draws as a Latin one, read as the
// letter it draws as, so a name can't pass for another by borrowing it:
// "Clear It" with a Cyrillic Es (round 12 on #6562, F170). The kit's ask
// row reads names through it, as LookForm.js's readAs does for For You.

/** Greek and Cyrillic letters that draw as a Latin one, and the fullwidth forms. */
const BORROWED = /[\u0391-\u03c7\u0405-\u04cf\u0501-\u051d\uff01-\uff5e]/gu;
/** Each borrowed Greek or Cyrillic letter, as the Latin one it draws as. */
const LOOKS_LATIN: Record<string, string> = {
  "\u0391": "a",
  "\u0392": "b",
  "\u0395": "e",
  "\u0396": "z",
  "\u0397": "h",
  "\u0399": "i",
  "\u039a": "k",
  "\u039c": "m",
  "\u039d": "n",
  "\u039f": "o",
  "\u03a1": "p",
  "\u03a4": "t",
  "\u03a5": "y",
  "\u03a7": "x",
  "\u03b1": "a",
  "\u03b3": "y",
  "\u03b9": "i",
  "\u03ba": "k",
  "\u03bd": "v",
  "\u03bf": "o",
  "\u03c1": "p",
  "\u03c4": "t",
  "\u03c5": "u",
  "\u03c7": "x",
  "\u0405": "s",
  "\u0406": "i",
  "\u0408": "j",
  "\u0410": "a",
  "\u0412": "b",
  "\u0415": "e",
  "\u041a": "k",
  "\u041c": "m",
  "\u041d": "h",
  "\u041e": "o",
  "\u0420": "p",
  "\u0421": "c",
  "\u0422": "t",
  "\u0423": "y",
  "\u0425": "x",
  "\u0430": "a",
  "\u0435": "e",
  "\u043e": "o",
  "\u0440": "p",
  "\u0441": "c",
  "\u0443": "y",
  "\u0445": "x",
  "\u0455": "s",
  "\u0456": "i",
  "\u0458": "j",
  "\u04bb": "h",
  "\u04c0": "l",
  "\u04cf": "l",
  "\u0501": "d",
  "\u051a": "q",
  "\u051b": "q",
  "\u051c": "w",
  "\u051d": "w",
};
/** The first fullwidth form, and how far each sits above its Latin letter. */
const FULLWIDTH_FIRST = 0xff_01;
const FULLWIDTH_FROM = 0xfe_e0;

/** A letter as the Latin one it draws as, where it's borrowed. */
function asLatin(each: string): string {
  const code = each.charCodeAt(0);
  if (code >= FULLWIDTH_FIRST) {
    return String.fromCharCode(code - FULLWIDTH_FROM);
  }
  return LOOKS_LATIN[each] ?? each;
}

/** A name with every letter it borrows read as the Latin one it draws as. */
function unborrowed(name: string): string {
  return name.replace(BORROWED, asLatin);
}

export { unborrowed };
