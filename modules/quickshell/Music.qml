pragma Singleton
// The music: whichever media player matters now, what it plays and where in
// the track, from any program that speaks MPRIS (Spotify, a browser's tab,
// mpv, spotify-player), so no player needs anything of its own. Which one
// matters is Music.js's to say.
//
// The media keys come here (`qs ipc call music toggle`, `next`, `previous`)
// rather than to playerctl, so they act on the player the shell shows. Each
// press shows the card (Notifications.qml), about the player it acted on,
// even while notices are held, since Joel asked for it; a track changing by
// itself shows it only while they aren't, so it never breaks into a call or
// a screen share, and never over the card a key just put up.
//
// Mpris says when a track or a state changes, so nothing is asked, but for
// the position, which it never reports: that's read once a second, only
// while the card is up and the music plays.
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

    // Whether the card is up, for a moment after it's called for.
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

    // A player's own change, which shows only while notices aren't held,
    // and never over what a key just did.
    function arrived(): void {
        if (playing && !Quiet.on && !(shown && told !== null))
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
    readonly property bool revealable: player !== null
        && (windowOf(player) !== undefined || player.canRaise)

    function windowOf(player: MprisPlayer): var {
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

    function toggle(): void {
        const acted = player;
        if (!acted)
            return;
        if (acted.isPlaying)
            acted.pause();
        else if (acted.canPlay)
            acted.play();
        show(acted);
    }

    function next(): void {
        const acted = player;
        if (acted && acted.canGoNext)
            acted.next();
        show(acted);
    }

    function previous(): void {
        const acted = player;
        if (acted && acted.canGoPrevious)
            acted.previous();
        show(acted);
    }

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
