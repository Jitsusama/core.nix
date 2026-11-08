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

// How a player stands, as the Media place's row says it at its far end,
// its state first as a timer's is (VKCF3Z, 2026-11-08): "1:38 of 6:43"
// playing, "paused \u00b7 1:38 of 6:43" paused, and a stream with no
// length only how far it's come.
function standing(position, length, playing) {
    const at = length > 0 ? `${time(position)} of ${time(length)}` : time(position);
    return playing ? at : `paused \u00b7 ${at}`;
}

// A place in a track as typed: seconds, as 90, or m:ss or h:mm:ss, as
// 1:30; null for anything else.
const CLOCK = /^(?:(\d+):)?(\d+):([0-5]\d)$/;
const SECONDS = /^\d+(?:\.\d+)?$/;
function placeIn(typed) {
    const text = String(typed ?? "").trim();
    if (SECONDS.test(text)) {
        return Number(text);
    }
    const clock = CLOCK.exec(text);
    if (!clock) {
        return null;
    }
    return Number(clock[1] ?? 0) * HOUR + Number(clock[2]) * 60 + Number(clock[3]);
}

// How a player loops, in Joel's words, by MPRIS's LoopStatus as Quickshell
// numbers it (None, Track, Playlist): MPRIS's None is Off to him (VKCF3Z,
// 2026-11-08).
const LOOPS = ["off", "track", "playlist"];
function loopOf(state) {
    return LOOPS[state] ?? "off";
}
function loopState(word) {
    return Math.max(0, LOOPS.indexOf(String(word ?? "").toLowerCase()));
}

// A speed as a player is asked it, "1\u00d7", in as few figures as say it.
function speed(rate) {
    return `${Number(rate.toFixed(2))}\u00d7`;
}

// The file a track's address names, as a person reads it, or "" where it's
// no file.
const FILE = /^file:\/\/.*\/([^/]+)$/u;
function fileOf(address) {
    const found = FILE.exec(String(address ?? ""));
    if (!found) {
        return "";
    }
    try {
        return decodeURIComponent(found[1]);
    } catch (_malformed) {
        return found[1];
    }
}

// Each player's note in the Media place: its artist, and where two players'
// rows would read alike, the first of its file's name or its window's
// workspace that tells them apart (VKCF3Z, 2026-11-08). Each player is
// `{ title, artist, file, workspace }`, and `onWorkspace` says a workspace
// with `{n}`.
function notes(players, onWorkspace) {
    return players.map(player => {
        const twins = players.filter(
            other => other.title === player.title && other.artist === player.artist,
        );
        const apart = field =>
            player[field] !== "" && new Set(twins.map(twin => twin[field])).size === twins.length;
        let told = "";
        if (twins.length > 1 && apart("file")) {
            told = player.file;
        } else if (twins.length > 1 && apart("workspace")) {
            told = onWorkspace.replace("{n}", player.workspace);
        }
        return [player.artist, told].filter(part => part !== "").join(" \u00b7 ");
    });
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
