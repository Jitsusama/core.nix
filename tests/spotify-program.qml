// Drives Spotify's program in the launcher (modules/spotify/
// SpotifyProgram.qml) through Quickshell itself, with spotify-recent stood
// in (tests/spotify-recent-stand-in.sh), and niri and what's playing too.
// Nothing is read while the launcher is closed; stepped into, Spotify is
// read once an opening, its three reads together, and again the next
// time it opens. Its Recent leads with what plays now, saying "playing"
// or "paused" as Spotify's player stands, the rest their kind and how
// long ago, their note who made them; its acts are Liked Songs, saying
// how many, and Playlists, each beside how many songs it holds. Inside a
// row are Play, Open in Spotify, then Copy the Link, its link Spotify's
// own for anyone; Return plays through the command while a Spotify runs,
// and otherwise, or when the command finds nowhere to play, opens the
// client on it. Signed out, its note says so and Sign In leads, running
// spotify-player's sign-in; out of reach, its note says that. It exits 0
// once every case holds, and otherwise 1, saying which didn't.
pragma ComponentBehavior: Bound

import Quickshell
import Quickshell.Io
import QtQuick
import "launcher" as Launcher

ShellRoot {
    id: root

    readonly property string home: Quickshell.env("HOME")
    readonly property string standIn: Quickshell.env("STAND_IN")

    property string waiting: "nothing"
    property var until: () => false
    property var then: () => {}

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    function same(what, got, wanted) {
        expect(what, JSON.stringify(got) === JSON.stringify(wanted),
            `${JSON.stringify(got)}, not ${JSON.stringify(wanted)}`);
    }

    function fail(message) {
        console.warn(`FAIL ${message}`);
        Qt.callLater(() => Qt.exit(1));
    }

    // Waits for `until` to hold, then goes on with `then`.
    function waitFor(what, until, then) {
        root.waiting = what;
        root.until = until;
        root.then = then;
        poll.start();
    }

    Timer {
        id: poll

        interval: 20
        repeat: true
        onTriggered: {
            try {
                if (!root.until())
                    return;
                poll.stop();
                root.then();
            } catch (error) {
                poll.stop();
                root.fail(error.message);
            }
        }
    }

    // Nothing here waits forever: a case that never comes fails, by name.
    Timer {
        running: true
        interval: 30000
        onTriggered: root.fail(`never came: ${root.waiting}`)
    }

    QtObject {
        id: shown

        property bool open: false
        property var held: []
    }

    // One for each way Spotify can stand, each the stand-in told so.
    component Spotify: Launcher.SpotifyProgram {
        id: spotify

        required property string answers

        launcher: shown
        settings: ({
            command: [root.standIn, spotify.answers],
            spotify: ["spotify"],
            signIn: ["kitty", "spotify_player", "authenticate"]
        })
    }

    Spotify {
        id: well

        answers: "well"
    }

    Spotify {
        id: signedOut

        answers: "signed-out"
    }

    Spotify {
        id: offline

        answers: "offline"
    }

    Spotify {
        id: nowhere

        answers: "nowhere"
    }

    // What the stand-in was asked, a line a run.
    FileView {
        id: asked

        path: `${root.home}/asked`
        printErrors: false
    }

    function askedOf(stands) {
        asked.reload();
        asked.waitForJob();
        return asked.text().split("\n").filter(line => line.startsWith(`${stands} `)).sort();
    }

    function names(rows) {
        return rows.map(row => row.name);
    }

    function sides(rows) {
        return rows.map(row => row.side);
    }

    Component.onCompleted: Qt.callLater(root.closed)

    // While the launcher is closed, nothing is read, however it's asked.
    function closed() {
        try {
            same("closed, Spotify gives no acts", well.programActs("spotify"), []);
            same("closed, Spotify has no Recent", well.recentOf("spotify"), []);
            same("closed, nothing is read", well.reading, false);
            same("another program is given nothing", well.programActs("slack"), []);
            shown.open = true;
            same("opened, nothing is read until Spotify is stepped into", well.reading, false);
            well.programActs("spotify");
            same("stepped into, it's read", well.reading, true);
            well.programActs("spotify");
            waitFor("Spotify read", () => well.recent.length === 3 && well.liked === 1234
                && well.playlists.length === 2, root.read);
        } catch (error) {
            fail(error.message);
        }
    }

    function read() {
        same("stepped into twice, Spotify is read once, all three together", askedOf("well"),
            ["well ", "well liked", "well playlists"]);
        const recent = well.recentOf("spotify");
        same("Recent, what plays now first", names(recent), ["Morning", "Two", "Liked Songs"]);
        same("beside each, playing, then its kind and how long ago, Liked Songs only when",
            sides(recent), ["playing", "album \u00b7 2 h ago", "2 h ago"]);
        same("its note, who made it", recent[1].note, "Singer, Guest");
        Launcher.Playing.player.isPlaying = false;
        same("what plays now says paused as Spotify's player does",
            well.recentOf("spotify")[0].side, "paused");
        Launcher.Playing.player.isPlaying = true;
        const acts = well.programActs("spotify");
        same("its acts, Liked Songs then Playlists", names(acts), ["Liked Songs", "Playlists"]);
        same("Liked Songs says how many", acts[0].side, "1,234 songs");
        same("Playlists says how many", acts[1].side, "2");
        const lists = acts[1].targets();
        same("every playlist inside", names(lists), ["Focus", "Road trip"]);
        same("each beside how many songs it holds", sides(lists), ["42 songs", "1 song"]);
        same("each its note, who made it", lists[0].note, "Joel");
        const inside = recent[0].verbs();
        same("inside a row, Play, Open in Spotify, then Copy the Link", names(inside),
            ["Play", "Open in Spotify", "Copy the Link"]);
        same("its link is Spotify's own", inside[2].copies,
            "https://open.spotify.com/playlist/p9");
        same("Liked Songs' link is anyone's own", acts[0].verbs()[2].copies,
            "https://open.spotify.com/collection/tracks");
        well.programActs("spotify");
        same("asked again this opening, nothing more is read", well.reading, false);
        inside[1].run();
        same("Open in Spotify opens the client on it", Niri.spawned,
            ["spotify", "spotify:playlist:p9"]);
        recent[1].run();
        waitFor("a play through the command", () => askedOf("well").length === 4, root.played);
    }

    function played() {
        same("while Spotify runs, Return plays through the command",
            askedOf("well").includes("well play spotify:album:a2"), true);
        same("and opens nothing", Niri.spawned, ["spotify", "spotify:playlist:p9"]);
        Launcher.Playing.players = [];
        well.recentOf("spotify")[1].run();
        same("with no Spotify running, Return opens the client on it", Niri.spawned,
            ["spotify", "spotify:album:a2"]);
        same("and asks nothing", askedOf("well").length, 4);
        Launcher.Playing.players = [Launcher.Playing.player];
        shown.open = false;
        shown.open = true;
        well.programActs("spotify");
        same("opened again, it's read again", well.reading, true);
        waitFor("Spotify read again as the launcher opens again",
            () => askedOf("well").length === 7, root.again);
    }

    function again() {
        same("opened again, it's read again", askedOf("well").filter(line => line === "well ")
            .length, 2);
        signedOut.programActs("spotify");
        offline.programActs("spotify");
        waitFor("Spotify signed out and out of reach", () => signedOut.stands === "signedOut"
            && offline.stands === "unreachable", root.refused);
    }

    function refused() {
        same("signed out, its note says so", signedOut.programNote("spotify"), "signed out");
        const acts = signedOut.programActs("spotify");
        same("and Sign In leads", acts.map(act => `${act.name} ${act.leads}`), ["Sign In true"]);
        same("with no Recent", signedOut.recentOf("spotify"), []);
        acts[0].run();
        same("Sign In runs spotify-player's sign-in", Niri.spawned,
            ["kitty", "spotify_player", "authenticate"]);
        same("out of reach, its note says so", offline.programNote("spotify"),
            "can't reach Spotify");
        same("and it has no acts", offline.programActs("spotify"), []);
        same("another program's note isn't Spotify's", signedOut.programNote("slack"), "");
        nowhere.programActs("spotify");
        waitFor("Spotify read with nowhere to play", () => nowhere.recent.length === 3,
            root.nowhereRead);
    }

    function nowhereRead() {
        nowhere.recentOf("spotify")[0].run();
        waitFor("the client opened on what had nowhere to play",
            () => Niri.spawned[1] === "spotify:playlist:p9", () => Qt.exit(0));
    }
}
