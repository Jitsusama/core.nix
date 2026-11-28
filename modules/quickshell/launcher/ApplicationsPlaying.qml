// What plays for Programs (Applications.qml): who plays and how it
// stands, heard from Mpris and PipeWire through Playing, which tells the
// provider to draw a program held from the top again, and each program's
// player as its Media row and drawer. Applications.qml keeps every name
// another file reads of this, its Sound among them, and makes this once.
import Quickshell
import qs
import QtQuick
import "Media.js" as Media

Scope {
    id: playingPart

    // Applications, whose programs these play for, which makes their rows.
    required property var provider

    // Held from the start, so the streams it binds say which program each
    // is before a program is first stepped into.
    readonly property var playing: Playing

    // How every player and stream stands, in what a program's Media and
    // Sound say of them, as one string. Mpris and PipeWire say when any of
    // it changes, so nothing is polled, and while something is stepped
    // into the launcher hears that this provider changed, which makes a
    // program held from the top again (Launcher.qml's refreshed): Pause
    // turns to Play when a media key pauses its player, Previous comes
    // once there's a track before, and the volume's bar moves with Louder.
    // A player's names and a stream's are in it too, since they may be
    // heard only once it's listed or bound, and they say whose it is
    // (playerOf, streamsOf, recordersOf).
    // JSON, since the names are other processes' and could hold any
    // separator, and one change that moved a name across it would say
    // nothing. Who plays is a binding of its own, so a title changing
    // makes it again only when a node did, and the handler reads it there.
    readonly property string playingWhoNow: playingWho()
    readonly property string playingNow: JSON.stringify([
        playingWhoNow,
        Playing.players.map(player => [player.isPlaying, player.canPlay, player.canPause,
            player.canGoNext, player.canGoPrevious, player.trackTitle, player.trackArtist]),
        Playing.streams.concat(Playing.recorders ?? []).map(node => [node.audio?.volume,
            node.audio?.muted])
    ])

    onPlayingNowChanged: {
        if (playingWhoNow !== playingHeard.who) {
            playingHeard.who = playingWhoNow;
            playingHeard.turn += 1;
            provider.startsTurn += 1;
        }
        provider.redrawHeld();
    }

    // Who plays: each player's and stream's names, all a closed program's
    // Media and Sound need to be there (startsOnly), not what plays or how
    // loud; a node's audio is there from the start for every audio node,
    // bound or not (Quickshell's PwNode, CONSTANT). A radio's titles
    // came every few seconds and would let every held answer go between
    // keys. Each change of who turns `turn`, which has each held answer
    // ask again only whether anything plays for its program (playsAsHeld),
    // since a chime's stream, an agent's done or a timer's ring, is no
    // program's. Plain, kept as last heard from the start (onCompleted),
    // so the first title after it isn't taken for a change of who; JSON,
    // as playingNow is.
    readonly property var playingHeard: ({ who: "", turn: 0 })
    function playingWho() {
        return JSON.stringify([
            Playing.players.map(player => [player.dbusName, player.desktopEntry,
                player.identity]),
            Playing.streams.concat(Playing.recorders ?? []).map(node => {
                const told = node.properties ?? {};
                return [node.id, node.name, told["application.name"],
                    told["application.process.binary"], told["node.link-group"],
                    told["stream.monitor"]];
            })
        ]);
    }

    // Its player that matters: the one playing, or else the first.
    function playerOf(same) {
        const players = Playing.players.filter(player => same(player.desktopEntry)
            || same(player.identity));
        return players.find(player => player.isPlaying) ?? players[0] ?? null;
    }

    // A track and who plays it, a dot between them, as every part of a
    // value is (VKCF3Z, 2026-11-08).
    function nowPlaying(player) {
        return player.trackArtist === "" ? player.trackTitle
            : `${player.trackTitle} \u00b7 ${player.trackArtist}`;
    }

    // Media: playing or pausing, one row by how it stands, then the next
    // and the last track, then what's playing. Next and Previous keep the
    // launcher, since they're pressed again and again, and the foot and
    // the last row say what's on now; a toggle is pressed once and goes.
    function media(player) {
        const row = provider.said("program:media", "", {
            hue: "program",
            // Everything the player says, under the row (Media.js).
            thing: true,
            drawer: () => playingPart.mediaDrawer(player),
            targets: () => {
                const rows = [];
                const going = player.isPlaying;
                if (going ? player.canPause : player.canPlay)
                    rows.push(provider.said("verb:program:play", going ? "playing" : "", {
                        stays: true,
                        run: () => {
                            if (player.isPlaying)
                                player.pause();
                            else
                                player.play();
                        }
                    }));
                if (player.canGoNext)
                    rows.push(provider.said("verb:program:next", "", {
                        stays: true,
                        state: () => playingPart.nowPlaying(player),
                        run: () => player.next()
                    }));
                if (player.canGoPrevious)
                    rows.push(provider.said("verb:program:previous", "", {
                        stays: true,
                        state: () => playingPart.nowPlaying(player),
                        run: () => player.previous()
                    }));
                if (player.trackTitle !== "")
                    rows.push(provider.figure("program:playing", () => player.trackTitle,
                        () => player.trackArtist, null));
                return rows;
            }
        });
        return provider.gotten(row, { side: () => player.isPlaying ? "playing" : "paused" });
    }

    // A player's drawer, where it is in its track read again each second
    // while one plays and the launcher shows, since MPRIS says a position
    // only when asked. Its art may be the network's, so only with lookups.
    function mediaDrawer(player) {
        void playedClock.date;
        return Media.drawer(player, id => Words.name(`program:media:${id}`),
            Words.of("program:media").said, provider.settings.lookups !== false);
    }

    SystemClock {
        id: playedClock
        enabled: !!playingPart.provider.launcher && playingPart.provider.launcher.open
            && Playing.players.some(player => player.isPlaying)
        precision: SystemClock.Seconds
    }
}
