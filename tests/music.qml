// Which player the music is, and how it's said, run in Qt's own JavaScript
// engine, the one the shell runs it in. The `the-music-follows-the-player`
// check copies Music.js beside this file and runs `qml -platform offscreen
// music.qml` over players made up here, shaped as MprisPlayer's are.
import QtQuick
import "Music.js" as Music

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    function run() {
        const failures = [];
        let asked = 0;
        const said = player => player ? player.dbusName.split(".").pop() : "nobody";
        const player = (name, playing, identity, trackTitle) => ({
            dbusName: `org.mpris.MediaPlayer2.${name}`,
            isPlaying: playing,
            identity: identity ?? name,
            trackTitle: trackTitle ?? ""
        });
        // Qt's engine has no Object.fromEntries.
        const at = times => Object.keys(times).reduce((all, name) =>
            Object.assign(all, { [`org.mpris.MediaPlayer2.${name}`]: times[name] }), {});
        const spotify = player("spotify", true);
        const youtube = player("chromium.instance412", true);
        const mpv = player("mpv", false);
        // playerctld repeats whichever player it saw last.
        const repeat = player("playerctld", true);

        // [what's on the bus, when each began playing, when each stopped,
        // who's current, and who else is playing]
        const cases = [
            ["one playing", [spotify], {}, {}, "spotify", []],
            ["the last to start wins", [spotify, youtube],
                at({ spotify: 1, "chromium.instance412": 2 }), {},
                "instance412", ["spotify"]],
            ["whatever order they're listed in", [youtube, spotify],
                at({ spotify: 2, "chromium.instance412": 1 }), {},
                "spotify", ["instance412"]],
            ["playing beats paused, however recent", [player("spotify", false), youtube],
                at({ spotify: 9, "chromium.instance412": 1 }), at({ spotify: 10 }),
                "instance412", []],
            ["nothing playing resumes the last to stop",
                [player("spotify", false), mpv],
                at({ mpv: 1, spotify: 5 }), at({ spotify: 7, mpv: 9 }),
                "mpv", []],
            ["nothing ever stopped resumes the last to start",
                [mpv, player("spotify", false)],
                at({ mpv: 1, spotify: 5 }), {},
                "spotify", []],
            ["playerctld is never the music", [repeat, mpv], {}, {}, "mpv", []],
            ["playerctld isn't a second player", [repeat, spotify], {}, {}, "spotify", []],
            ["nothing on the bus", [], {}, {}, "nobody", []]
        ];
        for (const [what, players, started, stopped, current, others] of cases) {
            asked++;
            const chosen = Music.current(players, started, stopped);
            const rest = Music.others(players, started, chosen).map(said);
            if (said(chosen) !== current || JSON.stringify(rest) !== JSON.stringify(others))
                failures.push(`${what}: chose ${said(chosen)} with ${JSON.stringify(rest)}, `
                    + `not ${current} with ${JSON.stringify(others)}`);
        }

        // [what, the player, who else is on the bus, what it's called]
        const tab = player("chromium.instance412", true, "Chromium", "Ocean Soul");
        const otherTab = player("chromium.instance977", true, "Chromium", "Ever Dream");
        const quiet = player("mpv.a", true, "mpv");
        const names = [
            ["its own name, alone", tab, [tab, spotify], "Chromium"],
            ["what it plays, beside one of the same name", tab, [tab, otherTab], "Ocean Soul"],
            ["its name when it plays nothing it names", quiet,
                [quiet, player("mpv.b", true, "mpv")], "mpv"],
            ["playerctld's echo isn't another of the name", spotify,
                [spotify, player("playerctld", true, "spotify")], "spotify"]
        ];
        for (const [what, one, players, expected] of names) {
            asked++;
            const got = Music.name(one, players);
            if (got !== expected)
                failures.push(`${what}: called "${got}", not "${expected}"`);
        }

        // [position, length, playing, said]
        const places = [
            [98.6, 403, true, "1:38 of 6:43"],
            [0, 213, true, "0:00 of 3:33"],
            [3725, 7200, true, "1:02:05 of 2:00:00"],
            [98, 403, false, "paused at 1:38"],
            [61, 0, true, "1:01"]
        ];
        for (const [position, length, playing, expected] of places) {
            asked++;
            const got = Music.where(position, length, playing);
            if (got !== expected)
                failures.push(`${position}s of ${length}s said "${got}", not "${expected}"`);
        }

        // When what's playing shows itself: [what, by a key, held, a key's
        // moment up, shows].
        const moments = [
            ["a key's press", true, false, false, true],
            ["a key's press while notices are held", true, true, false, true],
            ["a key's press over another key's", true, false, true, true],
            ["a track changing", false, false, false, true],
            ["a track changing while notices are held, as on a call", false, true, false, false],
            ["a track changing over what a key did", false, false, true, false]
        ];
        for (const [what, byKey, held, keyed, expected] of moments) {
            asked++;
            if (Music.calls(byKey, held, keyed) !== expected)
                failures.push(`${what} ${expected ? "didn't show" : "showed"}`);
        }

        // What a player's own change announces, once a track.
        const song = player("spotify", true, "Spotify", "Ocean Soul");
        const announcements = [
            ["a track playing", song, "", "org.mpris.MediaPlayer2.spotify Ocean Soul"],
            ["the track it said", song, "org.mpris.MediaPlayer2.spotify Ocean Soul", ""],
            ["the next track", song, "org.mpris.MediaPlayer2.spotify Ever Dream",
                "org.mpris.MediaPlayer2.spotify Ocean Soul"],
            ["a player paused", player("spotify", false, "Spotify", "Ocean Soul"), "", ""],
            ["no player", null, "", ""]
        ];
        for (const [what, one, announced, expected] of announcements) {
            asked++;
            const got = Music.announcement(one, announced);
            if (got !== expected)
                failures.push(`${what} announced "${got}", not "${expected}"`);
        }

        // The acts a player's row offers, Return's first.
        const able = (playing, can) => Object.assign(player("spotify", playing), {
            canPlay: can, canGoNext: can, canGoPrevious: can
        });
        const actsSaid = acts => acts.map(one => one.act + (one.state ? `:${one.state}` : "")
            + (one.other ? `:${said(one.other)}` : "")).join(" ");
        const offers = [
            ["playing", able(true, true), false, [], "toggle next previous"],
            ["paused, it plays", able(false, true), false, [], "toggle:paused next previous"],
            ["paused, and it can't play", able(false, false), false, [], ""],
            ["playing, it pauses though it can't go on", able(true, false), false, [], "toggle"],
            ["with somewhere to go and another playing", able(true, true), true, [youtube],
                "toggle next previous show pauseOther:instance412"],
            ["no player", null, true, [youtube], ""]
        ];
        for (const [what, one, revealable, rest, expected] of offers) {
            asked++;
            const got = actsSaid(Music.offered(one, revealable, rest));
            if (got !== expected)
                failures.push(`${what} offered "${got}", not "${expected}"`);
        }

        // What's playing as one row: its side the track, then where in it
        // where that's read; its note who plays it and the album, and the
        // player only where no act names it.
        const playing = (where, revealable) => ({
            title: "Ocean Soul", artist: "Nightwish", album: "Century Child", from: "Spotify",
            revealable, art: "file:///cover.jpg", where
        });
        const acts = [{ name: "Pause" }];
        const rows = [
            ["in Now", playing("", true),
                { side: [{ said: "Ocean Soul", keep: 2 }], note: "Nightwish \u00b7 Century Child" }],
            ["arriving", playing("1:38 of 6:43", true), {
                side: [{ said: "Ocean Soul", keep: 2 }, { said: "1:38 of 6:43", keep: 1 }],
                note: "Nightwish \u00b7 Century Child"
            }],
            ["with nowhere to go", playing("", false), {
                side: [{ said: "Ocean Soul", keep: 2 }],
                note: "Nightwish \u00b7 Century Child \u00b7 Spotify"
            }],
            ["with no artist or album", Object.assign(playing("", true), { artist: "", album: "" }),
                { side: [{ said: "Ocean Soul", keep: 2 }], note: "" }]
        ];
        for (const [what, one, expected] of rows) {
            asked++;
            const got = Music.row(one, "Playing", acts);
            const whole = Object.assign({
                key: "on:playing", kind: "playing", name: "Playing", glyph: "playing",
                hue: "program", picture: "file:///cover.jpg", acts
            }, expected);
            if (JSON.stringify(got) !== JSON.stringify(Object.assign({}, got, whole))
                    || Object.keys(got).length !== Object.keys(whole).length)
                failures.push(`the row ${what} read ${JSON.stringify(got)}`);
        }
        asked++;
        if (Music.row(null, "Playing", acts) !== null)
            failures.push("a row stands with nothing playing");

        // How the Media place's row stands: state first, as a timer's.
        const standings = [
            [98.6, 403, true, "1:38 of 6:43"],
            [98, 403, false, "paused \u00b7 1:38 of 6:43"],
            [61, 0, false, "paused \u00b7 1:01"]
        ];
        for (const [position, length, playing, expected] of standings) {
            asked++;
            const got = Music.standing(position, length, playing,
                { through: "{at} of {length}", paused: "paused \u00b7 {at}" });
            if (got !== expected)
                failures.push(`standing at ${position}s said "${got}", not "${expected}"`);
        }

        // Why a player didn't, as the launcher's other refused acts read,
        // in words.json's words; with no player, that nothing plays.
        const refusing = {
            refused: "{name} not {done} \u00b7 {why}",
            nothing: "nothing is playing",
            "done:shuffle": "shuffled",
            "done:place": "moved to {at}",
            "why:shuffle": "it can't shuffle",
            "why:place": "that isn't a time in a track"
        };
        const told = [
            [["Spotify", "shuffle", {}], "Spotify not shuffled \u00b7 it can't shuffle"],
            [["mpv", "place", { at: "soon" }],
                "mpv not moved to soon \u00b7 that isn't a time in a track"],
            [["", "shuffle", {}], "nothing is playing"]
        ];
        for (const [[name, act, fill], expected] of told) {
            asked++;
            const got = Music.refusal(refusing, name, act, fill);
            if (got !== expected)
                failures.push(`${act} refused said "${got}", not "${expected}"`);
        }

        // A place typed for :set position, in seconds or as a clock.
        const typed = [["90", 90], ["1:30", 90], ["1:02:05", 3725], ["2.5", 2.5],
            ["1:75", null], ["soon", null], ["", null]];
        for (const [text, expected] of typed) {
            asked++;
            const got = Music.placeIn(text);
            if (got !== expected)
                failures.push(`"${text}" read as ${got}, not ${expected}`);
        }

        // Looping in Joel's words, MPRIS's None being Off, and back.
        asked++;
        const states = ["off", "Track", "playlist", "none"].map(Music.loopState);
        if ([0, 1, 2].map(Music.loopOf).join() !== "off,track,playlist"
                || states.join() !== "0,1,2,0")
            failures.push("a loop isn't said as off, track and playlist");

        asked++;
        const sped = [1, 0.5, 1.25, 2.0001].map(Music.speed);
        if (sped.join() !== "1\u00d7,0.5\u00d7,1.25\u00d7,2\u00d7")
            failures.push("a speed isn't said in as few figures as say it");

        // A track's file, as a person reads it.
        if (Music.fileOf("file:///home/joel/Music/Bless%20The%20Child.mp3")
            !== "Bless The Child.mp3" || Music.fileOf("https://example.com/a.mp3") !== ""
            || Music.fileOf("file:///music/100%off.mp3") !== "100%off.mp3")
            failures.push("a track's file isn't read from its address");
        // One straight in the store goes without its hash; one in a folder
        // there is named as it is.
        if (Music.fileOf("file:///nix/store/bdmylmvw562fyhfd39fkn89zcw7qn66w-track.mp3")
            !== "track.mp3"
            || Music.fileOf("file:///nix/store/bdmylmvw562fyhfd39fkn89zcw7qn66w-album/a-b.mp3")
            !== "a-b.mp3")
            failures.push("a file in the store is named with its hash");

        // Two rows that would read alike are told apart by the file, else the
        // workspace, which is kept longest, a file giving way from its
        // middle; a row alone keeps its artist.
        const track = { title: "Bless The Child", artist: "Nightwish", workspace: "" };
        const written = notes => JSON.stringify(notes);
        const byFile = Music.notes([Object.assign({ file: "a.mp3" }, track),
            Object.assign({ file: "b.mp3" }, track)], "workspace {n}");
        if (written(byFile) !== written([
            [{ said: "Nightwish", keep: 1 }, { said: "a.mp3", keep: 2, file: "a.mp3" }],
            [{ said: "Nightwish", keep: 1 }, { said: "b.mp3", keep: 2, file: "b.mp3" }]]))
            failures.push(`two players on one track aren't told by file: ${written(byFile)}`);
        const bySpace = Music.notes([
            Object.assign({}, track, { file: "a.mp3", workspace: "1" }),
            Object.assign({}, track, { file: "a.mp3", workspace: "2" })], "workspace {n}");
        if (written(bySpace) !== written([
            [{ said: "Nightwish", keep: 1 }, { said: "workspace 1", keep: 2 }],
            [{ said: "Nightwish", keep: 1 }, { said: "workspace 2", keep: 2 }]]))
            failures.push(`two players on one file aren't told by workspace: ${written(bySpace)}`);
        const alone = Music.notes([Object.assign({ file: "a.mp3" }, track),
            { title: "Other", artist: "", file: "b.mp3", workspace: "" }], "w {n}");
        if (written(alone) !== written([[{ said: "Nightwish", keep: 1 }], []]))
            failures.push(`players apart already gain a note: ${written(alone)}`);

        asked++;
        if (Music.mark("Spotify") !== String.fromCodePoint(0xf1bc)
                || Music.mark("") !== String.fromCodePoint(0xf075a)
                || Music.mark(undefined) !== String.fromCodePoint(0xf075a))
            failures.push("a player's mark isn't its own, or a note when it has none");

        // Said as warnings, since Qt keeps ordinary logging quiet.
        for (const failure of failures)
            console.warn(failure);
        console.warn(failures.length > 0 ? `${failures.length} went wrong of ${asked}`
            : `all ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }
}
