// Which player the music is, and how it's said, for Music.qml. Kept apart
// from Quickshell's types, so the `the-music-follows-the-player` check can
// run it in Qt's own engine over players it makes up (tests/music.qml).
// A player here is anything with a dbusName and isPlaying, as an
// MprisPlayer has.
.pragma library

// playerctld's D-Bus name, which ends with its own.
const PLAYERCTLD = /\.playerctld$/;
// The seconds in an hour.
const HOUR = 3600;
// Nerd Fonts' note, the mark of a player it has none for.
const NOTE = 0xf_07_5a;

// playerctld speaks for whichever player it last saw, so it would be the
// same music twice.
function own(players) {
    return players.filter(player => !PLAYERCTLD.test(player.dbusName));
}

// Newest first, by when each did something, in milliseconds by D-Bus name.
function newest(when) {
    return (a, b) => (when[b.dbusName] ?? 0) - (when[a.dbusName] ?? 0);
}

// The one that matters: of those playing, the last to start, and when none
// is, the last to stop, which a play key resumes. `started` and `stopped`
// hold when each last began and last stopped playing.
function current(players, started, stopped) {
    const mine = own(players);
    const playing = mine.filter(player => player.isPlaying).sort(newest(started));
    return playing[0] ?? mine.sort(newest(started)).sort(newest(stopped))[0] ?? null;
}

// What to call a player in a list of them: its own name, or what it plays
// when another player goes by the same name, as two Chrome tabs do.
function name(player, players) {
    const shared = own(players).some(other => other !== player
        && other.identity === player.identity);
    return shared && player.trackTitle ? player.trackTitle : player.identity;
}

// Those playing but not the current one, last to start first.
function others(players, started, chosen) {
    return own(players).filter(player => player.isPlaying && player !== chosen)
        .sort(newest(started));
}

// m:ss, or h:mm:ss past the hour, as a player shows it.
function time(seconds) {
    const s = Math.max(0, Math.floor(seconds));
    const pad = n => String(n).padStart(2, "0");
    const hours = Math.floor(s / HOUR);
    const minutes = Math.floor(s / 60) % 60;
    return hours > 0 ? `${hours}:${pad(minutes)}:${pad(s % 60)}`
        : `${minutes}:${pad(s % 60)}`;
}

// Where in the track: "1:38 of 6:43", or "paused at 1:38". A stream with no
// length says only how far it's come.
function where(position, length, playing) {
    if (!playing) {
        return `paused at ${time(position)}`;
    }
    return length > 0 ? `${time(position)} of ${time(length)}` : time(position);
}

// A player's mark where Nerd Fonts has one, by the desktop entry it names,
// and a note otherwise.
const marks = {
    "spotify": 0xf1_bc,
    "google-chrome": 0xf2_68,
    "chromium": 0xf2_68,
    "chromium-browser": 0xf2_68,
    "firefox": 0xf2_69,
    "vlc": 0xf_05_7c
};

function mark(entry) {
    return String.fromCodePoint(marks[(entry ?? "").toLowerCase()] ?? NOTE);
}
