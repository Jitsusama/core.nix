// mpv's own in the launcher's Programs › mpv. It gives mpv's Recent the
// files mpv left (`recentOf`, which Applications.qml asks every provider
// that has it), each the one row every surface shows of a file
// (Files.qml), keyed by its path as Applications keys a file, so the
// same file from the list programs share is listed once, and dated by
// when mpv left it (`at`), so Recent orders it among the rest, newest
// first (Recent.js). One stopped part way says where as its note, as
// "12:04 of 41:10", and Return resumes it, as mpv does by itself, with
// Play From the Start inside; one watched whole says its length, and
// Return plays it from its start, since resuming would land on its last
// frame. Its drawer says when mpv left it and each figure of how far.
// mpv-stopped (stopped.sh) lists them as mpv is stepped into, once each
// time the launcher opens, never on a timer.
//
// Its act Play the Copied Link is there only while what was copied last
// is one address mpv plays (Copied.qml's `link`, narrowed by Mpv.js's
// playable to a video site's page or a media file, which yt-dlp and mpv
// take), its site beside it, and gone otherwise rather than dimmed; it's
// found from the top by its name too, as Meet's acts are.
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
    readonly property string link: Mpv.playable(Copied.link)
    // So Programs › mpv, held as a link is copied, gains it or loses it.
    onLinkChanged: changed()

    foundActs: link === "" ? [] : [{ program: provider.program, key: "verb:mpv:link" }]

    // The files mpv left, as mpv-stopped last listed them, each { path,
    // start, length, at }.
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
            side: Mpv.hostOf(address),
            note: address,
            run: () => Niri.spawn(Mpv.play(address, false))
        })];
    }

    function recentOf(asked) {
        return asked === program ? stopped.map(leftRow) : [];
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
    // part way, or played from its start once watched whole.
    function leftRow(one) {
        const said = Words.of("verb:mpv:on").said ?? {};
        const whole = Mpv.whole(one);
        const again = act("verb:mpv:start", one, {
            run: () => Niri.spawn(Mpv.play(one.path, true))
        });
        const on = act("verb:mpv:on", one, {
            side: (said.from ?? "").replace("{at}", Mpv.clock(one.start)),
            run: () => Niri.spawn(Mpv.play(one.path, false))
        });
        const row = Files.file(one.path, thing, whole ? [again] : [on, again]);
        const ago = Ago.ago(one.at, Date.now());
        const figure = (id, seconds) => ({ label: Words.name(id), value: Mpv.clock(seconds) });
        return Object.assign(row, {
            note: Mpv.where(one, whole ? said.whole : said.stopped),
            at: one.at,
            drawer: {
                eyebrow: ((whole ? said.watched : said.left) ?? "").replace("{ago}", ago),
                title: row.name,
                facts: (whole ? [] : [figure("mpv:stopped:at", one.start),
                    figure("mpv:stopped:to-go", Mpv.toGo(one))])
                    .concat([figure("mpv:stopped:length", one.length)])
            }
        });
    }
}
