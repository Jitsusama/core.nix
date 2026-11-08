// Media, a place: every media player that speaks MPRIS (Spotify, a
// browser's tab, mpv), what each plays and how, beside Sound, which holds the
// devices it plays through (VKCF3Z, 2026-11-08). It's launcher/Players.qml
// rather than Media.qml, since launcher/Media.js is a player's page. Each player is a row: its
// program's mark, the track as its name and the artist after it, and at the
// far end how it stands, "1:38 of 6:43" playing and "paused · 1:38 of 6:43"
// paused. Return plays or pauses it and the launcher stays. Its drawer is a
// player's page (Media.js, the same one Now's Playing row and a program's
// Media row draw).
//
// A step in, its acts, each only where the player can do it, never dimmed:
// Pause or Play, named for what it will do; Next; Previous; Position, a bar,
// stepping back or ahead or from the start; Volume, a bar and its share,
// louder, quieter or a level; Shuffle, on or off; Loop, Off, Track or
// Playlist; Speed, where it has more than one; Go To Its Window; and Copy
// the Link, last, where the track has one.
//
// The : line sets the player that matters now (Music.qml), as the media keys
// act on it: :set shuffle, noshuffle and shuffle!, loop=off|track|playlist,
// playervolume=40 with += and -=, and position=90 or 1:30 with += and -=
// to seek; volume is Sound's. On a player's row, or a step into it, :setl
// sets that player's own, as Vim's :setlocal sets a buffer's, and its acts
// name :setl. The command line has the same through `qs ipc call music`.
//
// MPRIS says when a player or what it plays changes, so nothing is asked but
// the position, which it never reports: read once a second while the place
// is open, through the clock the launcher's rows already tick by.
import qs
import Quickshell
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "../Music.js" as Players
import "Media.js" as Media

Provider {
    id: provider

    what: Words.of("media").what
    collection: Words.collection("media")

    // A tenth of the way, as a volume key steps; the levels a step into a
    // Volume offers, loudest first; the seconds ahead a step offers; the
    // speeds a step into a Speed offers, where the player goes that far.
    readonly property real volumeStep: 0.1
    readonly property var levels: [100, 90, 80, 70, 60, 50, 40, 30, 20, 10]
    readonly property var speeds: [0.5, 0.75, 1, 1.25, 1.5, 2]

    // The position's clock: the launcher asks again each second while the
    // place is open, since MPRIS never says the position moved.
    SystemClock {
        id: clock
        precision: SystemClock.Seconds
        enabled: provider.launcher?.open ?? false
        onDateChanged: if (Playing.players.some(player => player?.isPlaying)) provider.tick++
    }
    // Counted each second something plays, so the rows say where it's got.
    property int tick: 0

    function said(key, fill) {
        let text = Words.of("media").said?.[key] ?? key;
        for (const name of Object.keys(fill ?? {}))
            text = text.replace(`{${name}}`, fill[name]);
        return text;
    }

    // An act on a player, as the vocabulary names it, in the state it's
    // in, done to that player when it's run; the launcher stays, so it's
    // seen to change.
    function act(key, state, glyph, run, fill) {
        return thing({
            key: `${key}:${Object.values(fill ?? {}).join(":")}`,
            wordsId: key,
            wordState: state,
            verb: Words.verb(key, state),
            name: Words.name(key, state, fill ?? {}),
            glyph: glyph,
            hue: "action",
            stays: true,
            run: run
        });
    }

    // A level as a bar, which never warns.
    function bar(share) {
        return { shape: "bar", values: [Math.max(0, Math.min(1, share))], warn: false };
    }

    // A setting as a part: its name, how it stands and a bar, its choices a
    // step in and found from beside it by what's typed.
    function setting(fields, choices) {
        return holderRow(Object.assign({ offers: true }, fields), choices);
    }

    // Where a player is in its track, in seconds, read again whenever asked,
    // since MPRIS never says it moved.
    function secondsIn(player) {
        return Number(player?.position) || 0;
    }

    // A player as its note tells it from the rest: what it plays, the file
    // and the workspace its window is on.
    function about(player) {
        const window = Music.windowOf(player);
        const workspace = window ? Niri.workspaces[window.workspace_id]?.idx ?? "" : "";
        return {
            title: player.trackTitle,
            artist: player.trackArtist,
            file: Players.fileOf(player.metadata?.["xesam:url"]),
            workspace: String(workspace)
        };
    }

    // Its note as items, said apart from any other player's that would
    // read alike. The player is found again by its D-Bus name: a list read
    // back from a property is a fresh copy, whose players needn't be the
    // same objects, and one leaving may stand in it as null for a moment.
    function noteOf(player) {
        const players = Playing.players.filter(other => other);
        const notes = Players.notes(players.map(other => about(other)), said("workspace"));
        const index = players.findIndex(other => other.dbusName === player.dbusName);
        return notes[index] ?? [{ said: player.trackArtist, keep: 1 }];
    }

    function playerRow(player) {
        const loaded = player.trackTitle !== "";
        const position = secondsIn(player);
        const note = loaded ? noteOf(player) : [];
        return thing({
            thing: true,
            key: `player:${player.dbusName}`,
            // With nothing loaded, the program's own name (VKCF3Z).
            name: loaded ? player.trackTitle : player.identity,
            // Said whole where it's found by, and given way by its items
            // beside the row's whole side as it's drawn.
            count: note.map(item => item.said).join(" \u00b7 "),
            countItems: note,
            // How it stands is the row's need, so it's never cut (VKCF3Z).
            sideWhole: true,
            keywords: [player.identity, player.trackArtist, player.trackAlbum].join(" "),
            glyph: provider.programGlyph(player.desktopEntry),
            hue: "program",
            side: loaded || player.isPlaying
                ? Players.standing(position, player.length, player.isPlaying, said("paused"))
                : "",
            verb: Words.verb("verb:media:pause", player.isPlaying ? "" : "paused"),
            wordsId: "verb:media:pause",
            wordState: player.isPlaying ? "" : "paused",
            stays: true,
            now: true,
            run: () => provider.toggle(player),
            verbs: () => provider.parts(player),
            // Its own, which :setl sets here and a step in (CommandBinding.js).
            options: provider.playerOptions(player),
            drawer: () => {
                void clock.date;
                // Its position is said once, at the row's side, so the
                // drawer leads with its volume (VKCF3Z, 2026-11-08).
                const drawn = Media.drawer(player, id => Words.name(`program:media:${id}`),
                    Words.of("program:media").said, provider.settings.lookups !== false);
                const position = Words.name("program:media:position");
                return Object.assign({}, drawn, {
                    facts: (drawn.facts ?? []).filter(fact => fact.label !== position)
                });
            }
        });
    }

    function toggle(player) {
        if (player.isPlaying)
            player.pause();
        else if (player.canPlay)
            player.play();
        provider.changed();
    }

    // A player's acts, in the order VKCF3Z gave them (2026-11-08).
    function parts(player) {
        const rows = [];
        if (player.isPlaying ? player.canPause : player.canPlay)
            rows.push(act("verb:media:pause", player.isPlaying ? "" : "paused",
                player.isPlaying ? Theme.glyph.pause : Theme.glyph.resume,
                () => provider.toggle(player)));
        if (player.canGoNext)
            rows.push(act("verb:media:next", "", Theme.glyph.next, () => player.next()));
        if (player.canGoPrevious)
            rows.push(act("verb:media:previous", "", Theme.glyph.previous,
                () => player.previous()));
        if (player.canSeek && player.positionSupported)
            rows.push(positionPart(player));
        if (player.volumeSupported && player.canControl)
            rows.push(volumePart(player));
        if (player.shuffleSupported && player.canControl)
            rows.push(shufflePart(player));
        if (player.loopSupported && player.canControl)
            rows.push(loopPart(player));
        if (player.canControl && player.minRate < player.maxRate)
            rows.push(speedPart(player));
        if (Music.windowOf(player) !== undefined || player.canRaise)
            rows.push(thing({
                key: "verb:media:window",
                verb: Words.verb("verb:media:window"),
                name: Words.name("verb:media:window"),
                glyph: Words.glyph("verb:media:window"),
                hue: "action",
                later: true,
                run: () => Music.reveal(player)
            }));
        const link = String(player.metadata?.["xesam:url"] ?? "");
        if (link !== "")
            rows.push(copyRow({
                key: "verb:media:link",
                verb: Words.verb("verb:media:link"),
                name: Words.name("verb:media:link"),
                glyph: Words.glyph("verb:media:link"),
                hue: "action",
                figure: false,
                copy: link
            }));
        return rows;
    }

    function positionPart(player) {
        const position = secondsIn(player);
        const known = player.lengthSupported && player.length > 0;
        return setting({
            key: `player:${player.dbusName}:position`,
            name: Words.name("media:position"),
            side: Players.standing(position, player.length, true, said("paused")),
            glyph: Theme.glyph.position,
            graph: known ? bar(position / player.length) : null
        }, () => [act("verb:media:back", "", Theme.glyph.previous,
            () => Music.seekBy(player, -10))]
            .concat([act("verb:media:ahead", "", Theme.glyph.next,
                () => Music.seekBy(player, 10)),
                act("verb:media:further", "", Theme.glyph.next,
                () => Music.seekBy(player, 30))])
            .concat([act("verb:media:start", "", Theme.glyph.position,
                () => Music.seekTo(player, 0))]));
    }

    function volumePart(player) {
        const at = Math.round(player.volume * 100);
        return setting({
            key: `player:${player.dbusName}:volume`,
            name: Words.name("media:volume"),
            side: said("percent", { n: at }),
            glyph: Theme.glyph.volume,
            graph: bar(player.volume)
        }, () => [
            act("verb:media:louder", "", Theme.glyph.volume,
                () => Music.louden(player, player.volume + volumeStep)),
            act("verb:media:quieter", "", Theme.glyph.volume,
                () => Music.louden(player, player.volume - volumeStep))
        ].concat(levels.map(level => choice("media:volume:level", { n: level }, at === level,
            Theme.glyph.volume, () => Music.louden(player, level / 100)))));
    }

    // Shuffle, a switch: Return turns it the other way, as every switch
    // reads, on or off.
    function shufflePart(player) {
        const on = player.shuffle;
        return thing({
            key: `player:${player.dbusName}:shuffle`,
            wordsId: "media:shuffle",
            wordState: on ? "" : "off",
            name: Words.name("media:shuffle"),
            verb: Words.verb("media:shuffle", on ? "" : "off"),
            glyph: Theme.glyph.shuffle,
            // Muted as the settings beside it are, since a switch is one
            // (VKCF3Z, 2026-11-08).
            hue: "muted",
            side: said(on ? "on" : "off"),
            stays: true,
            run: () => Music.shuffled(player, !on)
        });
    }

    function loopPart(player) {
        const now = Players.loopOf(player.loopState);
        return setting({
            key: `player:${player.dbusName}:loop`,
            name: Words.name("media:loop"),
            side: Words.name(`media:loop:${now}`).toLowerCase(),
            glyph: Theme.glyph.loop
        }, () => ["off", "track", "playlist"].map(word => choice(`media:loop:${word}`, {},
            word === now, Theme.glyph.loop, () => Music.looped(player, word))));
    }

    function speedPart(player) {
        const offered = speeds.filter(rate => rate >= player.minRate && rate <= player.maxRate);
        return setting({
            key: `player:${player.dbusName}:speed`,
            name: Words.name("media:speed"),
            // The range it offers, never the player's own, which mpv gives
            // as 0.01× to 100×, speeds no one listens at.
            side: said("speeds", { speed: Players.speed(player.rate),
                least: Players.speed(offered[0] ?? player.minRate),
                most: Players.speed(offered[offered.length - 1] ?? player.maxRate) }),
            glyph: Theme.glyph.playSpeed
        }, () => offered.map(rate => choice("media:speed:choice", { speed: Players.speed(rate) },
            Math.abs(player.rate - rate) < 0.01, Theme.glyph.playSpeed,
            () => Music.sped(player, rate))));
    }

    // A choice a step into a setting, keyed by what it chooses too, so each
    // is a row of its own to hold as it becomes chosen.
    function choice(key, fill, chosen, glyph, run) {
        return thing({
            key: `${key}:${Object.values(fill).join(":")}`,
            wordsId: key,
            verb: Words.verb(key),
            name: Words.name(key, "", fill),
            glyph: glyph,
            hue: "action",
            side: chosen ? said("chosen") : "",
            stays: true,
            run: run
        });
    }

    // A player's options, which :setl sets on its row or a step into it,
    // and :set on the player that matters now, as the media keys act on
    // it (VKCF3Z, 2026-11-08). Each says why where the player can't.
    function playerOptions(player) {
        return {
            shuffle: {
                kind: "switch",
                now: () => player.shuffle,
                set: to => Music.shuffled(player, to)
            },
            loop: {
                kind: "choice",
                values: ["off", "track", "playlist"],
                now: () => Players.loopOf(player.loopState),
                set: to => Music.looped(player, to)
            },
            playervolume: {
                kind: "number",
                least: 0,
                most: 100,
                now: () => player.volumeSupported ? Math.round(player.volume * 100) : null,
                set: to => Music.louden(player, to / 100)
            },
            position: {
                kind: "number",
                least: 0,
                // Seconds, or a clock, as 1:30 (SetWords.js).
                clock: true,
                now: () => player.positionSupported
                    ? Math.round(provider.secondsIn(player)) : null,
                set: to => Music.seekTo(player, to)
            }
        };
    }

    // None while there's no player.
    setOptions: Music.player === null ? ({}) : playerOptions(Music.player)

    // A collection's rows are what it lists and what its search looks
    // through, as Sound's devices are.
    readonly property var all: {
        void tick;
        return Playing.players.filter(player => player).map(player => playerRow(player));
    }
    onAllChanged: changed()

    function search(typed) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const found = [];
        for (const row of all) {
            if ((row.prepared.mask & need) !== need)
                continue;
            const m = Fuzzy.match(row.prepared, words, false);
            if (m.score >= 0)
                found.push({ score: m.score, grade: m.grade, row: row });
        }
        let finest = 3;
        for (const f of found)
            finest = Math.min(finest, f.grade);
        const kept = found.filter(f => f.grade === finest).sort((a, b) => b.score - a.score);
        return { rows: kept.map(f => f.row), grade: finest };
    }

    function everything() {
        return all.slice();
    }
}
