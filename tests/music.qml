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
