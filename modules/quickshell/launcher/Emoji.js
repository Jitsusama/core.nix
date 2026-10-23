// Finding an emoji by what it's called or what it means. Each word typed has
// to start a word of its name ("thumbs" finds "thumbs up") or of one of its
// keywords ("tada" finds the party popper), and failing both, the name is
// matched loosely as everything else is (Fuzzy.js), for letters typed out of
// place. A name counts for more than a keyword, as in the launcher's main
// search, since it's what the emoji is, and a whole word for more than the
// start of one. Among names found as well, the one with fewest words left
// over ("cat" before "grinning cat"); then Unicode's own order, which puts
// the faces people use first ("grinning face" before "grinning cat"). A
// shorter name first would put the cat first there, and the man shrugging
// before the person. What's picked for what's typed is learned,
// so "lol" means the face with tears of joy after one pick, not the
// lollipop.
// Keywords are matched by where their words start, not loosely, since
// they're long and many: a loose match over all of them costs a frame.
.pragma library
.import "../Fuzzy.js" as Fuzzy

// What a word typed is worth, by where it was found.
const worth = { name: 40, nameStart: 30, keyword: 20, keywordStart: 10 };
// What the words' worth is multiplied by, so the least of it outweighs any
// number of a name's words left over, and a score from it is never below a
// loose match's.
const WORTH_SCALE = 10_000;
// What each word of a name left over takes from a score.
const LEFT_OVER = 100;
// What a loose match's score is divided by, to rank it below the rest.
const LOOSE_SCALE = 1000;
// What isn't a letter or digit, which leaves a space between words.
const NOT_WORD = /[^a-z0-9]+/g;

function spaced(text) {
    return ` ${text.toLowerCase().replace(NOT_WORD, " ").trim()} `;
}

// One emoji from a line of the built list, [emoji, name, keywords, group],
// with what a search needs worked out once: the words of its name and its
// keywords, each with a space before and after, so " " + a word typed finds
// where one starts and a space after finds a whole word, and every letter
// either holds.
function prepare(line, order) {
    const named = spaced(line[1]);
    const meant = spaced(line[2]);
    return {
        emoji: line[0],
        name: line[1],
        group: line[3],
        order,
        named,
        meant,
        words: named.trim().split(" ").length,
        mask: Fuzzy.letters([named, meant]),
        // The name ready to be matched loosely, worked out the first time
        // it's needed.
        loose: null
    };
}

// How well the words find an emoji, as worked out by `words`, or below
// zero when they don't.
function score(e, asked) {
    // biome-ignore lint/suspicious/noBitwiseOperators: a mask holds a bit for each letter.
    if ((e.mask & asked.need) !== asked.need) {
        return -1;
    }
    let total = 0;
    let named = 0;
    for (const word of asked.words) {
        if (e.named.indexOf(word.whole) >= 0) {
            total += worth.name;
            named += 1;
        } else if (e.named.indexOf(word.start) >= 0) {
            total += worth.nameStart;
            named += 1;
        } else if (e.meant.indexOf(word.whole) >= 0) {
            total += worth.keyword;
        } else if (e.meant.indexOf(word.start) >= 0) {
            total += worth.keywordStart;
        } else {
            return loosely(e, asked);
        }
    }
    if (named === 0) {
        return WORTH_SCALE * total;
    }
    return WORTH_SCALE * total - LEFT_OVER * Math.max(0, e.words - named);
}

// The name matched as everything else is, with no more than two pieces
// to a word, ranked below every match by where words start.
function loosely(e, asked) {
    if (e.loose === null) {
        e.loose = Fuzzy.prepare(e.name);
    }
    // biome-ignore lint/suspicious/noBitwiseOperators: a mask holds a bit for each letter.
    if ((e.loose.mask & asked.need) !== asked.need) {
        return -1;
    }
    const m = Fuzzy.match(e.loose, asked.typed, false);
    return m.score >= 0 && m.grade <= 1 ? m.score / LOOSE_SCALE : -1;
}

// What's typed, ready to score with: each word as the start of a word and as
// a whole one, and the letters they need.
function words(typed) {
    return {
        typed,
        need: Fuzzy.letters(typed),
        words: typed.map(w => ({ start: ` ${w}`, whole: ` ${w} ` }))
    };
}

// How scattered the best of what search found is, as Fuzzy.js grades a
// match: 0 when every word typed starts a word, 1 when it was found loosely.
function grade(found, typed) {
    return found.length > 0 && score(found[0], words(typed)) < WORTH_SCALE ? 1 : 0;
}

// Every emoji the words find, best first. `typed` is Fuzzy.words(what was
// typed); `among` the emoji to look in, which a longer query can narrow to
// those a shorter one found.
function search(among, typed) {
    if (typed.length === 0) {
        return [];
    }
    const asked = words(typed);
    const found = [];
    for (const e of among) {
        const s = score(e, asked);
        if (s >= 0) {
            found.push({ e, score: s });
        }
    }
    found.sort((a, b) => b.score - a.score || a.e.order - b.e.order);
    return found.map(f => f.e);
}
