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

// When `last` ({ key, words, at }, the last start that started) began, as
// hours and minutes, where `said` to the act `key` at `now` is the same
// start given again: the same act, the same words but for the space round
// them, within AGAIN_MS. Otherwise null, and it may start.
function startedAlready(last, key, said, now) {
    if (last === null || last === undefined || last.key !== key) {
        return null;
    }
    if (String(last.words).trim() !== String(said).trim() || now - last.at >= AGAIN_MS) {
        return null;
    }
    const at = new Date(last.at);
    const two = number => String(number).padStart(2, "0");
    return `${two(at.getHours())}:${two(at.getMinutes())}`;
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
