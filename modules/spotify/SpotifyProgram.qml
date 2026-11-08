// Spotify's program in the launcher (Programs › Spotify), from Spotify's
// Web API through spotify-recent (spotify-recent/src/main.rs), held to
// VKCF3Z's rulings. Its Recent (recentOf, which Applications.qml asks
// every provider that has it) is the playlists, albums and shows it
// played lately, each once, newest first, nine at most, what plays now
// first: beside each, "playing" or "paused" as Spotify's player stands,
// or how long ago it played, and as its note its kind and size, then who
// made it, as "playlist · 46 songs · Joel". Each row gives its time as
// `at`, in milliseconds, what plays now as this moment, so Recent.listed
// can lay it among others newest first. Return plays one, and inside are
// Play, Open in Spotify, then Copy the Link, Copy last as everywhere.
// Liked Songs and an artist are never in Recent, each having its home
// inside.
//
// Its insides one step in (programActs) are Liked Songs, saying how many,
// which plays them as a row of Recent plays, then Playlists, Albums,
// Artists and Shows, each saying how many it holds and holding every one
// Joel keeps, each the same row as Recent's, its note its size and who
// made it, found inside by typing. Each stands once it's been read, and
// says none while it holds nothing, so none comes and goes between
// openings.
//
// Return plays through the Web API (`spotify-recent play`) while a
// Spotify runs, its client or spotify-player, rather than asking the
// client over MPRIS (OpenUri), which isn't known to play a playlist
// rather than only show it: the API starts it where Spotify plays now,
// or on the first device open. With no Spotify running, or none the API
// can play on, Spotify's client is started on it (its %U), which shows
// it, since nothing could play it before the client is up.
//
// It's read only as Spotify is stepped into, once each time the launcher
// opens (programActs, which the launcher asks only then), never on a
// timer; what was read last stays shown until the new answer comes.
// While Spotify's sign-in is gone or Spotify can't be reached, its
// program row's note says so (programNote), and signed out its first act
// after Return's own is Sign In, as every signed-out source's is (VKCF3Z,
// 347abdb7), which runs spotify-player's sign-in in a terminal.
pragma ComponentBehavior: Bound

import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "Spotify.js" as Spotify

Provider {
    id: provider

    what: "program"

    // The desktop entry's id Spotify's program row is made from.
    readonly property string program: "spotify"
    // spotify-recent, Spotify's client and spotify-player's sign-in, each
    // as a command and its first words (home.nix).
    readonly property var recentCommand: settings.command ?? []
    readonly property var clientCommand: settings.spotify ?? ["spotify"]
    readonly property var signInCommand: settings.signIn ?? []

    // What was read last: what played lately, each list Joel keeps by its
    // name in spotify-recent's words (Spotify.js's LISTS), one not yet
    // read absent, and how many songs are liked, or -1 before they're
    // counted.
    property var recent: []
    property var lists: ({})
    property int liked: -1
    // How Spotify stood as it was last read (Spotify.js's standing), or
    // "" before it's read.
    property string stands: ""
    // Whether it's been read since the launcher opened, and how each read
    // this opening ended, by its exit status.
    property bool readNow: false
    property var ended: []
    // Whether a read this opening has yet to end, so nothing waits on a
    // read that never began.
    property bool reading: false
    // How many reads an opening makes: what played lately, the liked
    // songs' count and each list.
    readonly property int reads: 2 + Spotify.LISTS.length

    readonly property var said: Words.of("spotify:context").said ?? {}
    readonly property string note: Spotify.note(stands, said)
    // So Programs, read as the corpus is joined, says the note as it
    // stands now.
    onNoteChanged: changed()

    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (!provider.launcher.open)
                provider.readNow = false;
        }
    }

    // Reads it all, once an opening, and only while the launcher shows.
    function ask() {
        if (readNow || !(launcher?.open ?? false) || recentCommand.length === 0)
            return;
        readNow = true;
        ended = [];
        reading = true;
        recentReader.start(recentCommand);
        likedReader.start(recentCommand.concat(["liked"]));
        for (const reader of [playlistsReader, albumsReader, artistsReader, showsReader])
            reader.start(recentCommand.concat([reader.list]));
    }

    // How Spotify stands once a read ended with `code`: the worst any
    // read this opening said, so one answered after another was refused
    // still says why (Spotify.js's worst).
    function heard(code) {
        ended = ended.concat([code]);
        stands = Spotify.worst(ended);
        reading = ended.length < reads;
        changed();
    }

    // Spotify's player on the bus, its client's or spotify-player's, or
    // null while neither runs.
    function player() {
        return Playing.players.find(one => (one.dbusName ?? "")
            .startsWith("org.mpris.MediaPlayer2.spotify")) ?? null;
    }

    function programNote(app) {
        return app === program ? note : "";
    }

    function programActs(app) {
        if (app !== program)
            return [];
        ask();
        if (stands === "signedOut")
            return signInCommand.length === 0 ? [] : [signIn()];
        return (liked >= 0 ? [likedSongs()] : [])
            .concat(Spotify.LISTS.filter(name => name in lists).map(name => listed(name)));
    }

    function recentOf(app) {
        if (app !== program || stands === "signedOut")
            return [];
        const now = Date.now();
        return recent.map(record => Object.assign(row(record, provider.beside(record), true), {
            at: Spotify.when(record, now)
        }));
    }

    // Sign In, spotify-player's own sign-in in a terminal, which saves the
    // sign-in this reads.
    function signIn() {
        const key = "verb:spotify:sign-in";
        return thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            leads: true,
            later: true,
            run: () => Niri.spawn(provider.signInCommand)
        });
    }

    // Liked Songs, the account's own, saying how many, played and opened
    // as any of Recent's rows is.
    function likedSongs() {
        return row({
            uri: "spotify:collection:tracks",
            kind: "collection",
            name: Words.name("spotify:liked"),
            detail: "",
            at: "",
            size: ""
        }, Spotify.counted(liked, said.song ?? "", said.songs ?? ""), false);
    }

    // One of the lists Joel keeps, by its name, every one inside, saying
    // how many beside it, or none.
    function listed(name) {
        const key = `spotify:${name}`;
        const held = lists[name];
        return thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "program",
            side: held.length > 0 ? Spotify.grouped(held.length) : said.none ?? "",
            ask: "which?",
            nested: true,
            empty: said.none ?? "",
            targets: () => (provider.lists[name] ?? []).map(record => provider.row(record, "",
                false))
        });
    }

    // One playlist, album, artist, show or Liked Songs, one row wherever
    // it shows, keyed by its URI: beside it `side`, and as its note its
    // size and who made it, after its kind where `kinded`, as Recent mixes
    // kinds and a list holds one.
    function row(record, side, kinded) {
        const play = () => provider.play(record.uri);
        const ours = record.kind === "collection";
        return thing({
            key: `spotify:context:${record.uri}`,
            what: said[record.kind] ?? "",
            thing: true,
            of: "program:spotify",
            part: record.name,
            name: ours ? Words.name("spotify:liked") : record.name,
            verb: Words.verb("verb:spotify:play"),
            glyph: kindGlyph(record.kind),
            hue: "program",
            side: side,
            note: Spotify.noted(record, kinded, said),
            later: true,
            run: play,
            verbs: () => provider.acts(record, play)
        });
    }

    // The mark of a row's kind, the one its list inside wears, so a
    // playlist and an album tell apart at a glance in Recent, which mixes
    // them; a kind Spotify adds later wears what plays.
    function kindGlyph(kind) {
        if (kind === "collection")
            return Words.glyph("spotify:liked");
        return ["playlist", "album", "artist", "show"].includes(kind)
            ? Words.glyph(`spotify:${kind}s`) : Words.glyph("spotify:context");
    }

    // What Recent says beside a row, what plays now by Spotify's player
    // as it stands.
    function beside(record) {
        const live = player();
        return Spotify.beside(record, live ? live.isPlaying : null, Date.now(), said);
    }

    // Play, Open in Spotify, then Copy the Link, in the one order every
    // surface lists them.
    function acts(record, play) {
        const act = (key, more) => thing(Object.assign({
            key: `${key}:${record.uri}`,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action"
        }, more));
        const link = Spotify.link(record.uri);
        return [
            act("verb:spotify:play", { later: true, run: play }),
            act("verb:spotify:open", {
                later: true,
                run: () => Niri.spawn(provider.clientCommand.concat([record.uri]))
            }),
            act("verb:spotify:link", {
                now: true,
                copy: link,
                copies: link,
                run: () => {
                    Quickshell.clipboardText = link;
                }
            })
        ];
    }

    // Plays `uri` through the Web API while a Spotify runs, else starts
    // Spotify's client on it; one the API finds nowhere to play opens in
    // the client too.
    function play(uri) {
        if (player() === null || recentCommand.length === 0) {
            Niri.spawn(clientCommand.concat([uri]));
            return;
        }
        playReader.played = uri;
        playReader.start(recentCommand.concat(["play", uri]));
    }

    // One run of spotify-recent: what it printed and how it ended, each
    // null until it has, since Quickshell may say either first, then
    // `done` with both.
    component Reader: Process {
        id: reader

        property var printed: null
        property var ended: null

        signal done(int code, string text)

        function start(command) {
            if (reader.running)
                return;
            reader.printed = null;
            reader.ended = null;
            reader.command = command;
            reader.running = true;
        }

        function settle() {
            if (reader.printed !== null && reader.ended !== null)
                reader.done(reader.ended, reader.printed);
        }

        running: false
        onExited: code => {
            reader.ended = code;
            reader.settle();
        }
        stdout: StdioCollector {
            id: collected

            onStreamFinished: {
                reader.printed = collected.text;
                reader.settle();
            }
        }
    }

    Reader {
        id: recentReader

        onDone: (code, text) => {
            if (code === 0)
                provider.recent = Spotify.records(text);
            provider.heard(code);
        }
    }

    Reader {
        id: likedReader

        onDone: (code, text) => {
            if (code === 0)
                provider.liked = Number(text.trim());
            provider.heard(code);
        }
    }

    // A list's reader, keeping what it read under the list's name, one of
    // Spotify.js's LISTS.
    component ListReader: Reader {
        id: listReader

        required property string list

        onDone: (code, text) => {
            if (code === 0) {
                const kept = Object.assign({}, provider.lists);
                kept[listReader.list] = Spotify.records(text);
                provider.lists = kept;
            }
            provider.heard(code);
        }
    }

    ListReader {
        id: playlistsReader

        list: "playlists"
    }

    ListReader {
        id: albumsReader

        list: "albums"
    }

    ListReader {
        id: artistsReader

        list: "artists"
    }

    ListReader {
        id: showsReader

        list: "shows"
    }

    // Playing, which falls back to the client when the API has nowhere to
    // play, and says it's signed out when it is.
    Reader {
        id: playReader

        property string played: ""

        onDone: code => {
            const now = Spotify.standing(code);
            if (now === "nowhere")
                Niri.spawn(provider.clientCommand.concat([playReader.played]));
            else if (now === "signedOut")
                provider.stands = now;
        }
    }
}
