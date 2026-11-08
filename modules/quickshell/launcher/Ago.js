// How long ago something was, in the fewest words, the same wherever the
// launcher says it: a copy, a notice, the clock's last sync. Plain
// JavaScript, so each place says it one way.
.pragma library

const SECOND = 1000;
const MINUTE = 60;
const HOUR = 3600;
const DAY = 86_400;

// How long from `at` to `now`, both in milliseconds, in the largest unit
// it's whole in: "12 min", "15 h", "3 d", never less than a minute, as
// Needs You says how long an item has waited.
function span(at, now) {
    const seconds = Math.max(0, (now - at) / SECOND);
    if (seconds < HOUR) {
        return `${Math.max(1, Math.floor(seconds / MINUTE))} min`;
    }
    if (seconds < DAY) {
        return `${Math.floor(seconds / HOUR)} h`;
    }
    return `${Math.floor(seconds / DAY)} d`;
}

// How long before `now` the moment `at` was, both in milliseconds.
function ago(at, now) {
    return (now - at) / SECOND < MINUTE ? "just now" : `${span(at, now)} ago`;
}
