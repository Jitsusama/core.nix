// mpv's own in the launcher's Programs › mpv. Its Recent leads with the
// files mpv stopped part way through (`recentOf`, which Applications.qml
// asks every provider that has it), each the one row every surface shows
// of a file (Files.qml), saying where it stopped before its folder, as
// "12:04 of 41:10 · ~/Videos", and when in its note. Return plays on from
// there, as mpv does by itself with a file it stopped, and inside is Play
// From the Start too. mpv-stopped (stopped.sh) lists them as mpv is
// stepped into, once each time the launcher opens, never on a timer, and
// each row says when it was left (`at`), so Recent can order it among the
// rest, newest first, and leads (`leads`) until it does.
//
// Its first act after Return's own is Play the Copied Link, while what was
// copied last (Copied.qml) is one address mpv plays, a video site's page or
// a media file (Mpv.js's playable), its site beside it; it's found from the
// top by its name too, as Meet's acts are.
import qs
import Quickshell.Io
import QtQuick
import "Ago.js" as Ago
import "Mpv.js" as Mpv

Provider {
    id: provider

    // mpv's desktop entry, whose program these are given to.
    readonly property string program: "mpv"

    // The address mpv would play from the clipboard, or "".
    readonly property string link: Mpv.playable(Copied.text)
    // So Programs › mpv, held as a link is copied, gains it or loses it.
    onLinkChanged: changed()

    foundActs: link === "" ? [] : [{ program: provider.program, key: "verb:mpv:link" }]

    // The files mpv stopped part way through, as mpv-stopped last listed
    // them, each { path, start, length, at }.
    property var stopped: []

    // Whether mpv's own step is held, from Programs or from the top, and
    // whether its files were read since the launcher opened.
    readonly property bool steppedIn: (launcher?.open ?? false)
        && (launcher?.held ?? []).some(step => step.row?.key === `program:${provider.program}`)
    property bool lookedSince: false

    onSteppedInChanged: {
        if (!steppedIn || lookedSince || (settings.stopped ?? []).length === 0)
            return;
        lookedSince = true;
        listing.running = true;
    }

    // The next opening reads them again, as they may have changed since.
    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (!provider.launcher.open)
                provider.lookedSince = false;
        }
    }

    Process {
        id: listing

        command: provider.settings.stopped ?? []
        running: false
        stdout: StdioCollector {
            id: listed

            onStreamFinished: {
                provider.stopped = Mpv.stopped(listed.text.split("\n")
                    .filter(line => line !== ""));
                provider.changed();
            }
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
            leads: true,
            side: Mpv.hostOf(address),
            note: address,
            run: () => Niri.spawn(Mpv.play(address, false))
        })];
    }

    function recentOf(asked) {
        return asked === program ? stopped.map(stoppedRow) : [];
    }

    // One file mpv stopped part way through, as its Recent's row.
    function stoppedRow(one) {
        const said = Words.of("verb:mpv:on").said ?? {};
        const on = thing({
            key: `verb:mpv:on:${one.path}`,
            name: Words.name("verb:mpv:on"),
            verb: Words.verb("verb:mpv:on"),
            glyph: Words.glyph("verb:mpv:on"),
            hue: "action",
            side: (said.from ?? "").replace("{at}", Mpv.clock(one.start)),
            run: () => Niri.spawn(Mpv.play(one.path, false))
        });
        const again = thing({
            key: `verb:mpv:start:${one.path}`,
            name: Words.name("verb:mpv:start"),
            verb: Words.verb("verb:mpv:start"),
            glyph: Words.glyph("verb:mpv:start"),
            hue: "action",
            run: () => Niri.spawn(Mpv.play(one.path, true))
        });
        const where = Mpv.where(one, said.stopped);
        return Object.assign(Files.held(one.path, thing, [on, again], where), {
            note: (said.left ?? "").replace("{ago}", Ago.ago(one.at, Date.now())),
            at: one.at,
            leads: true
        });
    }
}
