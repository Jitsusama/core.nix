// How long ago something was, in the fewest words, the same wherever the
// launcher says it: a copy, a notice, the clock's last sync. Plain
// JavaScript, so each place says it one way.
.pragma library

const SECOND = 1000;
const MINUTE = 60;
const HOUR = 3600;
const DAY = 86_400;

// How long before `now` the moment `at` was, both in milliseconds.
function ago(at, now) {
    const seconds = Math.max(0, (now - at) / SECOND);
    if (seconds < MINUTE)
        return "just now";
    if (seconds < HOUR)
        return `${Math.floor(seconds / MINUTE)} min ago`;
    if (seconds < DAY)
        return `${Math.floor(seconds / HOUR)} h ago`;
    return `${Math.floor(seconds / DAY)} d ago`;
}
