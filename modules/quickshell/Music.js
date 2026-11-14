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

// Whether what's playing shows itself for a moment, on the arrival line: a
// media key's press (`byKey`) always, even while notices are held, since
// Joel asked for it; a player's own change only while they aren't
// (`held`, Quiet.on, which a shared screen turns on), so it never breaks
// into a call or a share, and never while a key's moment is up (`keyed`),
// which keeps saying what the key did.
function calls(byKey, held, keyed) {
    return byKey || (!held && !keyed);
}

// What a player's own change says of it, its player and its track, so it
// shows once for each track a player plays, whichever order MPRIS says it
// took over, began playing and changed track in; "" where it says nothing
// new: no player, one not playing, or the track it last said (`announced`).
function announcement(player, announced) {
    if (!player || !player.isPlaying) {
        return "";
    }
    const now = `${player.dbusName} ${player.trackTitle}`;
    return now === announced ? "" : now;
}

// The acts a player's row offers, Return's first, each as the act it is
// (`act`) and words.json's state for its words (`state`): play where it's
// paused and can, else pause; the next and the previous track where it can
// go to them; going to it where it has somewhere to go (`revealable`); and
// pausing each other player playing beside it (`others`), as `other`.
function offered(player, revealable, others) {
    if (!player) {
        return [];
    }
    const paused = !player.isPlaying;
    return [].concat(
        !paused || player.canPlay ? [{ act: "toggle", state: paused ? "paused" : "" }] : [],
        player.canGoNext ? [{ act: "next", state: "" }] : [],
        player.canGoPrevious ? [{ act: "previous", state: "" }] : [],
        revealable ? [{ act: "show", state: "" }] : [],
        (others ?? []).map(other => ({ act: "pauseOther", state: "", other })));
}

// What's playing as a launcher row, in the ready-row form a daemon serves
// its rows in (name, note, side, glyph, hue, acts), so Now's Playing row
// and the arrival line draw the one row. `track` says it: its `title`,
// `artist`, `album`, its player (`from`), whether an act names that player
// (`revealable`), its cover (`art`) and `where` in it, "" where it isn't
// read. Its name is Now's for it (`name`); its side the track and where in
// it, the track kept longest; its note who plays it, the album, and the
// player once, unless an act names it. Null with nothing playing.
function row(track, name, acts) {
    if (!track) {
        return null;
    }
    const side = [{ said: track.title, keep: 2 }]
        .concat(track.where ? [{ said: track.where, keep: 1 }] : []);
    return {
        key: "on:playing",
        kind: "playing",
        name,
        side,
        note: [track.artist, track.album, track.revealable ? "" : track.from]
            .filter(said => said).join(" \u00b7 "),
        glyph: "playing",
        hue: "program",
        picture: track.art ?? "",
        acts
    };
}

// How a player stands, as the Media place's row says it at its far end,
// its state first as a timer's is (VKCF3Z, 2026-11-08): "1:38 of 6:43"
// playing, and paused as `paused` says it, in words.json's media said
// (`through`, "{at} of {length}", and `paused`, "paused \u00b7 {at}"), and a
// stream with no length only how far it's come.
function standing(position, length, playing, said) {
    const at = length > 0
        ? said.through.replace("{at}", time(position)).replace("{length}", time(length))
        : time(position);
    return playing ? at : said.paused.replace("{at}", at);
}

// Why a player didn't do what it was asked, as the launcher's other refused
// acts read (VKCF3Z): "Spotify not shuffled \u00b7 it can't shuffle", from
// words.json's media said, `act` naming its done:act and its why:act, and
// `fill` what's in their braces. With no player (`who` empty), that
// nothing plays.
function refusal(said, who, act, fill) {
    if (who === "") {
        return said.nothing ?? "";
    }
    let text = (said.refused ?? "").replace("{name}", who)
        .replace("{done}", said[`done:${act}`] ?? act)
        .replace("{why}", said[`why:${act}`] ?? "");
    for (const key of Object.keys(fill ?? {})) {
        text = text.replace(`{${key}}`, fill[key]);
    }
    return text;
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
// no file. Read without a pattern: Qt's engine matched a top-level one
// only the first time the shell asked, and "" ever after, so two players
// on one track were never told apart. A file straight in the Nix store
// is named without its hash and the dash after it, "track.mp3" (VKCF3Z,
// 2026-11-08).
const FILE = "file://";
const STORE = "file:///nix/store/";
const HASHED = 33;
function fileOf(address) {
    const given = String(address ?? "");
    const folder = given.slice(0, given.lastIndexOf("/") + 1);
    const whole = given.slice(folder.length);
    const last = folder === STORE && whole.charAt(HASHED - 1) === "-"
        ? whole.slice(HASHED) : whole;
    if (!given.startsWith(FILE) || last === "") {
        return "";
    }
    try {
        return decodeURIComponent(last);
    } catch (_malformed) {
        return last;
    }
}

// Each player's note in the Media place, as items a row gives way by
// whole from its end (Choice.qml's countItems): its artist, and where two
// players' rows would read alike, the first of its file's name or its
// window's workspace that tells them apart, which is kept longest, a
// file's name giving way from its middle (VKCF3Z, 2026-11-08). Each
// player is `{ title, artist, file, workspace }`, and `onWorkspace` says
// a workspace with `{n}`.
function notes(players, onWorkspace) {
    return players.map(player => {
        const twins = players.filter(
            other => other.title === player.title && other.artist === player.artist,
        );
        const apart = field =>
            player[field] !== "" && new Set(twins.map(twin => twin[field])).size === twins.length;
        let told = null;
        if (twins.length > 1 && apart("file")) {
            told = { said: player.file, keep: 2, file: player.file };
        } else if (twins.length > 1 && apart("workspace")) {
            told = { said: onWorkspace.replace("{n}", player.workspace), keep: 2 };
        }
        return [{ said: player.artist, keep: 1 }, told]
            .filter(item => item !== null && item.said !== "");
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
