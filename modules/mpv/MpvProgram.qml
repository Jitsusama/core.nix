// mpv's own in the launcher's Programs › mpv. It gives mpv's Recent the
// files mpv left (`recentOf`, which Applications.qml asks every provider
// that has it), each the one row every surface shows of a file
// (Files.qml), keyed by its path as Applications keys a file, so the
// same file from the list programs share is listed once, and dated by
// when mpv left it (`at`), so Recent orders it among the rest, newest
// first (Recent.js). One stopped part way says beside it where, its
// length and when, as "12:04 of 41:10 · just now", and Return resumes
// it, as mpv does by itself, with Play From the Start inside; one watched
// whole says its length and when, and Return plays it from its start,
// since resuming would land on its last frame. Its drawer (Mpv.js's
// plate) says where it is and its kind, how much is still to play, what
// mpv kept for it, its size and its path. mpv-stopped (stopped.sh) lists
// them as mpv is stepped into, once each time the launcher opens, never
// on a timer, run by the programs daemon (Listed.qml), so the shell
// starts no process for them.
//
// Its act Play the Copied Link is there only while what was copied last
// is one address mpv plays (Copied.qml's `link`, narrowed by Mpv.js's
// playable to a video site's page or a media file, which yt-dlp and mpv
// take), its site beside it, and gone otherwise rather than dimmed; it's
// found from the top by its name too, as Meet's acts are.
import qs
import QtQuick
import "Mpv.js" as Mpv

Provider {
    id: provider

    // mpv's desktop entry, whose program these are given to.
    readonly property string program: "mpv"

    // The address mpv would play from the clipboard, or "".
    readonly property string link: Mpv.playable(Copied.link)
    // So Programs › mpv, held as a link is copied, gains it or loses it.
    onLinkChanged: changed()

    foundActs: link === "" ? [] : [{ program: provider.program, key: "verb:mpv:link" }]

    // The files mpv left, as mpv-stopped last listed them, each { path,
    // start, length, at, size, kind, kept }.
    property var stopped: []

    // What its rows and drawers say, the launcher's words, and days and
    // months as the locale names them, as the launcher says when
    // everywhere (Y2YBTS, 2026-11-03).
    readonly property var words: Mpv.wordsOf(id => Words.of(id),
        (at, format) => Qt.formatDate(at, format))

    // Whether mpv's own step is held, from Programs or from the top, and
    // whether its files were read since the launcher opened.
    readonly property bool steppedIn: (launcher?.open ?? false)
        && (launcher?.held ?? []).some(step => step.row?.key === `program:${provider.program}`)
    property bool lookedSince: false

    onSteppedInChanged: {
        if (steppedIn)
            lookedSince = true;
    }

    // The next opening reads them again, as they may have changed since.
    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (!provider.launcher.open)
                provider.lookedSince = false;
        }
    }

    // What mpv-stopped printed, as the programs daemon holds it under the
    // key its settings name, from the first step in an opening.
    Listed {
        id: listing

        key: provider.settings.stopped ?? ""
        wanted: provider.lookedSince

        onHeard: {
            provider.stopped = Mpv.stopped(listing.lines);
            provider.changed();
        }
    }

    function programActs(asked) {
        if (asked !== program || link === "")
            return [];
        const key = "verb:mpv:link";
        const address = link;
        return [thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            side: Mpv.hostOf(address),
            note: address,
            run: () => Niri.spawn(Mpv.play(address, false))
        })];
    }

    function recentOf(asked) {
        return asked === program ? stopped.map(leftRow) : [];
    }

    // What its drawer in Programs says of what it left part way
    // (Applications.qml asks every provider's programFacts), from what
    // mpv-stopped said when mpv was last stepped into.
    function programFacts(asked) {
        if (asked !== program)
            return [];
        return Mpv.held(stopped, words.said)
            .map(fact => ({ label: Words.name(fact.id), value: fact.value }));
    }

    // One of mpv's acts on a file, by its key in the launcher's words.
    function act(key, one, fields) {
        return thing(Object.assign({
            key: `${key}:${one.path}`,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action"
        }, fields));
    }

    // One file mpv left, as its Recent's row: resumed where it stopped
    // part way, or played from its start once watched whole. What the
    // shared list's row says of it, its folder, its drawer's eyebrow says
    // instead, so its side has room for where and when.
    function leftRow(one) {
        const said = words.said;
        const whole = Mpv.whole(one);
        const again = act("verb:mpv:start", one, {
            run: () => Niri.spawn(Mpv.play(one.path, true))
        });
        const on = act("verb:mpv:on", one, {
            side: (said.from ?? "").replace("{at}", Mpv.clock(one.start)),
            run: () => Niri.spawn(Mpv.play(one.path, false))
        });
        const row = Files.file(one.path, thing, whole ? [again] : [on, again]);
        const shown = {
            name: row.name,
            folder: Files.within(one.path),
            path: Files.shown(one.path)
        };
        return Object.assign(row, {
            side: Mpv.side(one, Date.now(), words),
            at: one.at,
            drawer: Mpv.plate(one, shown, words)
        });
    }
}
