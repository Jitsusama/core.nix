pragma Singleton
pragma ComponentBehavior: Bound
// The music: whichever media player matters now, what it plays and where in
// the track, from any program that speaks MPRIS (Spotify, a browser's tab,
// mpv, spotify-player), so no player needs anything of its own. Which one
// matters is Music.js's to say.
//
// The media keys come here (`qs ipc call music toggle`, `next`, `previous`)
// rather than to playerctl, so they act on the player the shell shows. Each
// press shows what's playing for a moment (`arrival`), about the player it
// acted on, even while notices are held, since Joel asked for it; a track
// changing by itself shows it only while they aren't, so it never breaks
// into a call or a screen share, and never over what a key just put up
// (Music.js's calls). It's one row, the one Now's Playing row is (`row`),
// which the arrival line draws, and until it does the stack's card.
//
// Mpris says when a track or a state changes, so nothing is asked, but for
// the position, which it never reports: that's read once a second, only
// while the moment is up and the music plays.
import qs
import Quickshell
import Quickshell.Io
import Quickshell.Services.Mpris
import QtQuick
import "Music.js" as Players

Singleton {
    id: root

    // When each player last began and last stopped playing, by its D-Bus
    // name: the last to start is the one that matters, and with nothing
    // playing, the last to stop is the one a play key resumes.
    property var started: ({})
    property var stopped: ({})
    readonly property MprisPlayer player:
        Players.current(Mpris.players.values, started, stopped)
    // Any others playing at the same time, which the piece offers to pause.
    readonly property var others: Players.others(Mpris.players.values, started, player)
    readonly property bool playing: player !== null && player.isPlaying

    readonly property string title: player ? player.trackTitle : ""
    readonly property string artist: player ? player.trackArtist : ""
    readonly property string album: player ? player.trackAlbum : ""
    readonly property string art: player ? player.trackArtUrl : ""
    readonly property string from: player ? player.identity : ""
    readonly property string mark: Players.mark(player ? player.desktopEntry : "")

    // What to call another player playing, beside this one.
    function name(other: MprisPlayer): string {
        return Players.name(other, Mpris.players.values);
    }

    // What Now calls it, the name of its row on every surface.
    readonly property string heading: "Playing"

    // What's playing as Now's row says it, in the ready-row form
    // (Music.js's row): the track, who plays it and on what, and its acts,
    // play or pause first; null with no player. Where in the track isn't
    // on it, since that's read only while the moment is up; Now's drawer
    // says it, read as its own clock turns.
    readonly property var row: Players.row(trackOf(player, ""), heading, actsOf(player))
    // The same row, with where in the track, while what's playing shows
    // itself for a moment, of the player a key acted on; null otherwise.
    // The arrival line draws it, and until it does the stack's card.
    readonly property var arrival: shown && featured !== null
        ? Players.row(trackOf(featured, card.where), heading, actsOf(featured)) : null

    // Lets the moment go before its time, as a click on it does.
    function dismiss(): void {
        hide.stop();
        shown = false;
        told = null;
    }

    // A player as a row says it, with where in the track as given.
    function trackOf(about: MprisPlayer, where: string): var {
        return about === null ? null : {
            title: about.trackTitle,
            artist: about.trackArtist,
            album: about.trackAlbum,
            from: about.identity,
            revealable: revealableOf(about),
            art: about.trackArtUrl,
            where: where
        };
    }

    // The chord bound to a music command, beside the act that does it.
    function chord(command: string): var {
        return (Shortcuts.actions.find(action => action.run.join(" ")
            === `qs ipc call music ${command}`) ?? { chord: [] }).chord;
    }

    // A player's acts as a row holds them (Piece.qml's actions), each in
    // words.json's words for how it stands, and each acting on that
    // player. Only the one that matters now offers to pause the others.
    function actsOf(about: MprisPlayer): var {
        return Players.offered(about, revealableOf(about), about === player ? others : [])
            .map(one => actOf(about, one));
    }

    function actOf(about: MprisPlayer, one: var): var {
        const id = `verb:on:playing:${one.act === "toggle" ? "pause" : one.act}`;
        if (one.act === "show")
            return {
                glyph: Players.mark(about.desktopEntry),
                name: Words.name(id, "", { player: about.identity }),
                verb: Words.verb(id),
                run: () => root.reveal(about)
            };
        if (one.act === "pauseOther") {
            const called = root.name(one.other);
            return {
                glyph: Words.glyph(id),
                name: Words.name(id, "", { player: called }),
                // The player isn't the row's, so the : line fills its
                // :pause {player} from here, as the name is.
                fill: { player: called },
                verb: Words.verb(id),
                stays: true,
                run: () => one.other.pause()
            };
        }
        return {
            glyph: Words.glyph(id, one.state),
            name: Words.name(id, one.state),
            verb: Words.verb(id, one.state),
            chord: chord(one.act),
            stays: true,
            run: () => one.act === "toggle" ? root.toggleOf(about)
                : one.act === "next" ? root.nextOf(about) : root.previousOf(about)
        };
    }

    // Whether what's playing is showing itself, for a moment after it's
    // called for.
    property bool shown: false
    // How long it stays: long enough to read a title and an artist.
    readonly property int moment: 4000
    // The player a key acted on, which the card follows while it's up
    // though another may matter now: pausing one of two hands the music to
    // the other, and the card says what the key did.
    property MprisPlayer told: null
    readonly property MprisPlayer featured: shown && told !== null ? told : player
    property real position: 0
    // What the card says, of the player it's about.
    readonly property var card: featured === null ? null : {
        title: featured.trackTitle,
        artist: featured.trackArtist,
        art: featured.trackArtUrl,
        from: featured.identity,
        mark: Players.mark(featured.desktopEntry),
        where: Players.where(position, featured.length, featured.isPlaying)
    }

    function show(about: MprisPlayer): void {
        told = about;
        position = featured ? featured.position : 0;
        shown = true;
        hide.restart();
    }

    // What last showed itself, as its player and track.
    property string announced: ""

    // A player's own change, which shows only while notices aren't held,
    // and never over what a key just did. It shows once for each track a
    // player plays, so a player starting shows itself every time.
    function arrived(): void {
        // Read from the player itself: when it goes, this can run before
        // playing and title have heard it's gone.
        const now = Players.announcement(root.player, announced);
        if (now === "")
            return;
        announced = now;
        if (Players.calls(false, Quiet.on, shown && told !== null))
            show(null);
    }

    function began(player: MprisPlayer): void {
        started = Object.assign({}, started, { [player.dbusName]: Date.now() });
    }

    function ended(player: MprisPlayer): void {
        stopped = Object.assign({}, stopped, { [player.dbusName]: Date.now() });
    }

    // Whether the player has something to bring forward: a window niri
    // holds, named as its desktop entry is, or a way to raise its own. One
    // with neither, mpv playing only sound, has nothing to show, and
    // starting its program would only open a second player.
    readonly property bool revealable: revealableOf(player)

    function revealableOf(about: MprisPlayer): bool {
        return about !== null && (windowOf(about) !== undefined || about.canRaise);
    }

    function windowOf(player: MprisPlayer): var {
        if (!player)
            return undefined;
        const entry = (player.desktopEntry ?? "").toLowerCase();
        return entry === "" ? undefined : Object.values(Niri.windows)
            .find(window => (window.app_id ?? "").toLowerCase() === entry);
    }

    // Brings a player's window forward.
    function reveal(player: MprisPlayer): void {
        const window = windowOf(player);
        if (window)
            Niri.act({ FocusWindow: { id: window.id } });
        else if (player.canRaise)
            player.raise();
    }

    // The media keys, on the player that matters now, and a row's acts, on
    // the player it's of. Each is a key's press, which shows what it did
    // whether notices are held or not (Music.js's calls).
    function toggle(): void {
        toggleOf(player);
    }

    function next(): void {
        nextOf(player);
    }

    function previous(): void {
        previousOf(player);
    }

    function toggleOf(acted: MprisPlayer): void {
        if (!acted)
            return;
        if (acted.isPlaying)
            acted.pause();
        else if (acted.canPlay)
            acted.play();
        show(acted);
    }

    function nextOf(acted: MprisPlayer): void {
        if (acted && acted.canGoNext)
            acted.next();
        show(acted);
    }

    function previousOf(acted: MprisPlayer): void {
        if (acted && acted.canGoPrevious)
            acted.previous();
        show(acted);
    }

    // What else a player can be asked, by the Media place (launcher/
    // Players.qml), its :set options and the command line, each only where
    // the player says it can, so none does nothing without a word. Each
    // says nothing where it did, or why it didn't, for whoever asked, in
    // words.json's words (refused).
    function refused(player: MprisPlayer, act: string, fill: var): string {
        return Players.refusal(Words.of("media").said ?? {}, player ? player.identity : "", act,
            fill);
    }

    function louden(player: MprisPlayer, share: real): string {
        if (!player || !player.volumeSupported || !player.canControl)
            return refused(player, "volume", {});
        player.volume = Math.max(0, Math.min(1, share));
        return "";
    }

    function shuffled(player: MprisPlayer, on: bool): string {
        if (!player || !player.shuffleSupported || !player.canControl)
            return refused(player, on ? "shuffle" : "order", {});
        player.shuffle = on;
        return "";
    }

    function looped(player: MprisPlayer, word: string): string {
        if (!player || !player.loopSupported || !player.canControl)
            return refused(player, word === "off" ? "unloop" : "loop", {});
        player.loopState = Players.loopState(word);
        return "";
    }

    // A step through the track, back or ahead, in seconds.
    function seekBy(player: MprisPlayer, seconds: real): string {
        if (!player || !player.canSeek)
            return refused(player, "seek", {});
        player.seek(seconds);
        position = player.position;
        return "";
    }

    // A place in the track, in seconds from its start.
    function seekTo(player: MprisPlayer, seconds: real): string {
        if (!player || !player.canSeek || !player.positionSupported)
            return refused(player, "seek", {});
        player.position = Math.max(0, player.lengthSupported
            ? Math.min(seconds, player.length) : seconds);
        position = player.position;
        return "";
    }

    function sped(player: MprisPlayer, rate: real): string {
        if (!player || !player.canControl || player.minRate >= player.maxRate)
            return refused(player, "speed", {});
        player.rate = Math.max(player.minRate, Math.min(player.maxRate, rate));
        return "";
    }

    // The keys and the command line: each acts on the player that matters
    // now, as the media keys do, and says nothing but what went wrong.
    IpcHandler {
        target: "music"

        function toggle(): void {
            root.toggle();
        }
        function next(): void {
            root.next();
        }
        function previous(): void {
            root.previous();
        }
        // Its volume, as a percentage.
        function volume(percent: int): string {
            return root.louden(root.player, percent / 100);
        }
        // A step through the track, as 10 or -10 seconds.
        function seek(seconds: real): string {
            return root.seekBy(root.player, seconds);
        }
        // A place in the track, as 90 or 1:30.
        function position(at: string): string {
            const seconds = Players.placeIn(at);
            return seconds === null ? root.refused(root.player, "place", { at: at })
                : root.seekTo(root.player, seconds);
        }
        function shuffle(on: bool): string {
            return root.shuffled(root.player, on);
        }
        // off, track or playlist.
        function loop(word: string): string {
            return root.looped(root.player, word);
        }
    }

    // Each player as it comes, so its starting is heard.
    Instantiator {
        model: Mpris.players

        Connections {
            required property MprisPlayer modelData
            target: modelData

            function onIsPlayingChanged() {
                if (modelData.isPlaying)
                    root.began(modelData);
                else
                    root.ended(modelData);
            }

            Component.onCompleted: {
                if (modelData.isPlaying)
                    root.began(modelData);
            }
        }
    }

    // A new track, or another player taking over, shows itself unless
    // notices are held.
    Connections {
        target: root.player

        function onPostTrackChanged() {
            root.arrived();
        }
    }

    onPlayerChanged: arrived()
    onPlayingChanged: arrived()

    Timer {
        id: hide
        interval: root.moment
        onTriggered: {
            root.shown = false;
            root.told = null;
        }
    }

    Timer {
        running: root.shown && root.featured !== null && root.featured.isPlaying
        interval: 1000
        repeat: true
        onTriggered: root.position = root.featured.position
    }
}
