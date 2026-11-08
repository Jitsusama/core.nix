// What a media player's drawer says, run in Qt's own JavaScript engine, the
// one the shell runs it in. The `a-player-says-all-it-knows` check copies
// Music.js, launcher/Media.js and Pictures.js beside this file and runs
// `qml -platform offscreen media.qml` over players made up here, shaped as
// Quickshell's MprisPlayer is.
import QtQuick
import "launcher/Media.js" as Media
import "Pictures.js" as Pictures

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

    // The vocabulary's words for the values, as words.json's media gives
    // them, and each fact named by its id, so a fact's order is read off.
    readonly property var said: ({
        eyebrow: "on {player} \u00b7 track",
        on: "on",
        off: "off",
        track: "track",
        playlist: "playlist",
        disc: "disc {disc}",
        range: "{least} to {most}"
    })

    function named(id) {
        return `[${id}]`;
    }

    // A player saying everything MPRIS can, as Spotify's does but the rate.
    function full() {
        return {
            identity: "Spotify",
            dbusName: "org.mpris.MediaPlayer2.spotify",
            trackTitle: "Ocean Soul",
            trackArtist: "Nightwish",
            trackAlbum: "Century Child",
            trackAlbumArtist: "Nightwish",
            trackArtUrl: "https://i.scdn.co/image/ab67616d",
            positionSupported: true,
            position: 98.6,
            lengthSupported: true,
            length: 403,
            volumeSupported: true,
            volume: 0.64,
            shuffleSupported: true,
            shuffle: false,
            loopSupported: true,
            loopState: 2,
            rate: 1,
            minRate: 0.5,
            maxRate: 2,
            canSetFullscreen: false,
            metadata: {
                "xesam:albumArtist": ["Tarja Turunen"],
                "xesam:trackNumber": 3,
                "xesam:discNumber": 1,
                "xesam:genre": ["Symphonic metal", "Power metal"],
                "xesam:userRating": 0.8,
                "xesam:url": "https://open.spotify.com/track/2xyz",
                "xesam:asText": "  Deep in the ocean\n"
            }
        };
    }

    function run() {
        const failures = [];
        let asked = 0;
        const expect = (what, got, want) => {
            asked++;
            if (JSON.stringify(got) !== JSON.stringify(want))
                failures.push(`${what}: ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
        };

        const spotify = Media.drawer(full(), named, said, true);
        expect("the eyebrow is where it plays and what it is", spotify.eyebrow,
            "on Spotify \u00b7 track");
        expect("the title is the track", spotify.title, "Ocean Soul");
        expect("its art is the preview, square", spotify.preview,
            { picture: "https://i.scdn.co/image/ab67616d", across: 0, down: 0 });
        expect("its lyrics are its words", spotify.body, "Deep in the ocean");
        // What moves first, then what describes, then what identifies.
        expect("every fact, what moves most first", spotify.facts, [
            { label: "[position]", value: "1:38 of 6:43" },
            { label: "[volume]", value: "64%" },
            { label: "[shuffle]", value: "off" },
            { label: "[loop]", value: "playlist" },
            { label: "[rate]", value: "1\u00d7 \u00b7 0.5\u00d7 to 2\u00d7" },
            { label: "[artist]", value: "Nightwish" },
            { label: "[album]", value: "Century Child" },
            { label: "[album-artist]", value: "Tarja Turunen" },
            { label: "[track]", value: "3 \u00b7 disc 1" },
            { label: "[genre]", value: "Symphonic metal, Power metal" },
            { label: "[rating]", value: "80%" },
            { label: "[address]", value: "https://open.spotify.com/track/2xyz" },
            { label: "[bus-name]", value: "org.mpris.MediaPlayer2.spotify" }
        ]);

        // A browser's tab says little: no volume, shuffle, loop or rate it
        // can change, a local picture, and no album artist but the artist.
        const tab = Object.assign(full(), {
            identity: "Chromium",
            dbusName: "org.mpris.MediaPlayer2.chromium.instance412",
            trackArtUrl: "file:///tmp/.org.chromium.Chromium.abc",
            volumeSupported: false,
            shuffleSupported: false,
            loopSupported: false,
            minRate: 1,
            maxRate: 1,
            canSetFullscreen: true,
            fullscreen: true,
            metadata: { "xesam:albumArtist": "Nightwish" }
        });
        const tabbed = Media.drawer(tab, named, said, false);
        expect("what a player can't do or doesn't say is left out",
            tabbed.facts.map(fact => fact.label),
            ["[position]", "[fullscreen]", "[artist]", "[album]", "[bus-name]"]);
        expect("a local picture shows with lookups off", tabbed.preview?.picture,
            "file:///tmp/.org.chromium.Chromium.abc");
        expect("no words when it gives none", tabbed.body, "");

        expect("a remote picture isn't asked for with lookups off",
            Media.drawer(full(), named, said, false).preview, null);
        expect("a picture by another scheme isn't drawn",
            Media.art("blob:abc", true), "");

        // A stream with no length says how far it's come; a player that
        // can't say where it is says nothing of it.
        const stream = Object.assign(full(), { length: 0, position: 61 });
        expect("a stream says how far it's come",
            Media.drawer(stream, named, said, true).facts[0].value, "1:01");
        const nowhere = Object.assign(full(), { positionSupported: false });
        expect("no position where it can't say one",
            Media.drawer(nowhere, named, said, true).facts[0].label, "[volume]");
        // A track with no title is named by its player, and one with no
        // track number still says its disc.
        const untitled = Object.assign(full(), { trackTitle: " ",
            metadata: { "xesam:discNumber": 2 } });
        const unnamed = Media.drawer(untitled, named, said, true);
        expect("a track with no title is its player's", unnamed.title, "Spotify");
        expect("a disc alone", unnamed.facts.find(fact => fact.label === "[track]")?.value,
            "disc 2");
        expect("a rate off one, to two places", Media.rate(1.256), "1.26\u00d7");

        // A picture is drawn from a path as a file, and a URL as given.
        expect("a path is a file", Pictures.source("/run/user/1000/shot.png"),
            "file:///run/user/1000/shot.png");
        expect("a URL is as given", Pictures.source("https://i.scdn.co/image/a"),
            "https://i.scdn.co/image/a");
        expect("a file URL isn't a file twice", Pictures.source("file:///tmp/a.png"),
            "file:///tmp/a.png");
        expect("nothing is nothing", Pictures.source(undefined), "");

        // Said as warnings, since Qt keeps ordinary logging quiet.
        for (const failure of failures)
            console.warn(failure);
        console.warn(failures.length > 0 ? `${failures.length} went wrong of ${asked}`
            : `all ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }
}
