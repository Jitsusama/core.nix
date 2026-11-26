.pragma library

// What Start New Work says of a start under the words that began it. On
// 25 November Joel had "no confirmation that it took my enter ... and now
// no clue if i did it once, twice, thrice or not at all", so a start counts
// its wait out loud, and the same words given again say when they started
// rather than starting the work twice.

const SECOND = 1000;
const MINUTE = 60;

// How long the same words given the same act again count as the start
// already made, an hour: past it, work said the same way is likely new.
const AGAIN_MS = 60 * MINUTE * SECOND;

// How long after Start New Work said a start was made already a Return
// with the same words starts it once more, two minutes: he was just told,
// so giving them again is meant. Past it, he's told again first.
const MEANT_MS = 2 * MINUTE * SECOND;

// How soon after he was told a Return counts as meant, a second: one held,
// or pressed twice, lands sooner, and would start the work twice.
const HEARD_MS = SECOND;

// How many starts are kept to say again, more than an hour's work holds.
const MOST_MADE = 12;

// Whether two starts' words are the same but for the space round them.
function same(words, said) {
    return String(words).trim() === String(said).trim();
}

// Whether two starts are one: the same act, the same words and the same
// place. Every From a Link act shares one key, so the same words for the
// cloud, or for here, are other work.
function alike(one, asked) {
    return one.key === asked.key && same(one.words, asked.words)
        && String(one.on ?? "") === String(asked.on ?? "");
}

// The starts made (`list`, each { key, words, on, at, answered, toldAt }),
// the start `given` ({ key, words, on }, its act, his words and where it
// was asked to start) at `now` and ended
// `how` (Sprouting.ended) among them, in place of any made before with the
// same words. One counts where it started, or where nothing answered,
// since that one may still have started; a refused one began nothing, so
// the start made before it stays. Those an hour old go.
function made(list, given, how, now) {
    const recent = (Array.isArray(list) ? list : []).filter(one => now - one.at < AGAIN_MS);
    if (how?.started !== true && how?.unanswered !== true) {
        return recent;
    }
    const kept = recent.filter(one => !alike(one, given));
    const start = { key: given.key, words: String(given.words), on: String(given.on ?? ""),
        at: now, answered: how.started === true, toldAt: 0 };
    return kept.concat([start]).slice(-MOST_MADE);
}

// The start among `list` that `asked` ({ key, words, on }) at `now` makes
// again, to be told rather than started: the same act, words and place
// (alike), within AGAIN_MS, unless he was told of it between HEARD_MS and
// MEANT_MS ago. Otherwise null, and it may start.
function again(list, asked, now) {
    const meant = one => one.toldAt > 0 && now - one.toldAt >= HEARD_MS
        && now - one.toldAt < MEANT_MS;
    return (Array.isArray(list) ? list : []).find(one => alike(one, asked)
        && now - one.at < AGAIN_MS && !meant(one)) ?? null;
}

// The start among those under way, `running` (each { key, words, on }:
// asked of the node, or started and its session still coming up), that
// `asked` gives again, or null: the same act, words and place (alike), so
// it keeps the receipt it already says rather than being told it's busy or
// started already over it (Joel, 2026-11-25 14:33: "no clue if i did it
// once, twice, thrice"). Another page's words on the same act, or the
// same words for the cloud, are new work.
function underWayAgain(running, asked) {
    return (Array.isArray(running) ? running : []).find(one => alike(one, asked)) ?? null;
}

// `list` with `toldOne` marked told at `now`, so a Return just after starts it.
function told(list, toldOne, now) {
    return list.map(one => (one === toldOne ? Object.assign({}, one, { toldAt: now }) : one));
}

// When the start `one` was asked, as hours and minutes.
function clock(one) {
    const asked = new Date(one.at);
    const two = number => String(number).padStart(2, "0");
    return `${two(asked.getHours())}:${two(asked.getMinutes())}`;
}

// How long a start under way has waited, from `since` to `now`, both in
// milliseconds, to the second, since a wait counted by the minute looks
// stopped: "8 s", then "1 min 05 s". Nothing before it began.
function waited(since, now) {
    if (!(since > 0)) {
        return "";
    }
    const seconds = Math.max(0, Math.floor((now - since) / SECOND));
    if (seconds < MINUTE) {
        return `${seconds} s`;
    }
    const left = String(seconds % MINUTE).padStart(2, "0");
    return `${Math.floor(seconds / MINUTE)} min ${left} s`;
}
