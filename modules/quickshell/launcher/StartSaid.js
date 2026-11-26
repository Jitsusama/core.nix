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

// How many starts are kept to say again, more than an hour's work holds.
const MOST_MADE = 12;

// Whether two starts' words are the same but for the space round them.
function same(words, said) {
    return String(words).trim() === String(said).trim();
}

// The starts made (`list`, each { key, words, at, answered, toldAt }), the
// start `given` ({ key, words }, its act and his words) at `now` and ended
// `how` (Sprouting.ended) among them, in place of any made before with the
// same words. One counts where it started, or where nothing answered,
// since that one may still have started; a refused one began nothing.
// Those an hour old go.
function made(list, given, how, now) {
    const kept = (Array.isArray(list) ? list : []).filter(one => now - one.at < AGAIN_MS
        && !(one.key === given.key && same(one.words, given.words)));
    if (how?.started !== true && how?.unanswered !== true) {
        return kept;
    }
    const start = { key: given.key, words: String(given.words), at: now,
        answered: how.started === true, toldAt: 0 };
    return kept.concat([start]).slice(-MOST_MADE);
}

// The start among `list` that `said` given to the act `key` at `now` makes
// again, to be told rather than started: the same act, the same words,
// within AGAIN_MS, unless he was told of it within MEANT_MS. Otherwise
// null, and it may start.
function again(list, key, said, now) {
    return (Array.isArray(list) ? list : []).find(one => one.key === key
        && same(one.words, said) && now - one.at < AGAIN_MS
        && !(one.toldAt > 0 && now - one.toldAt < MEANT_MS)) ?? null;
}

// `list` with `told` marked told at `now`, so a Return just after starts it.
function told(list, toldOne, now) {
    return list.map(one => (one === toldOne ? Object.assign({}, one, { toldAt: now }) : one));
}

// When the start `one` was asked, as hours and minutes.
function at(one) {
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
