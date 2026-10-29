// Capture, a collection, and Print's: a picture or a video of the screen, a
// window, a region, or from the camera, so one key means capturing anything
// and the rest is chosen here, found by typing what it's of. A camera glyph
// says a picture comes of it and the recording glyph a video; the words say
// what of. The whole screen comes first, so Print then Return takes it, and
// while anything records, stopping it comes first instead, so the gesture
// that started it stops it. Each row shows its own chord where it has one,
// to learn by use and never to need.
//
// Typing a window's title finds it on any workspace, since niri takes a
// window's picture wherever it is. A region is picked by the keys
// (Picker.qml), and Escape there comes back here. The camera's rows open its
// viewfinder (modules/camera), and are left out where there's no camera.
// Each waits for the launcher to be gone, so it's never in the picture.
// Whether a camera is open, and each camera's parts, are the cameras' own
// (Cameras.qml). After its own rows comes its Recent: what was taken
// lately, screenshots, recordings and the camera's pictures and videos in
// one list, newest first, nine at most, each the file it is wherever it
// shows (Files.qml), a picture shown in the drawer, read each time Capture
// is stepped into and never otherwise (VKCF3Z and Y2YBTS, 2026-10-29).
// It holds nothing another program lodges: editing the newest screenshot
// and looking through them all are GIMP's and the picture viewer's own, in
// their programs (VKCF3Z, 2026-10-29), and a picture taken holds what each
// program that edits it gives (`fileActs`), as Edit in GIMP.
import Quickshell
import Quickshell.Io
import qs
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    what: "capture"

    // What it captures are others' things, a window found by its title
    // among them, so the search lists them where they're their own: here
    // they'd be the same window twice (Joel, 2026-10-23).
    collection: Words.collection("capture", { searched: false })

    readonly property string picture: Theme.glyph.capture

    // The desktop's bind that runs what a row runs. A niri action's command
    // ends its options with --, which says nothing here.
    function bind(run) {
        return Shortcuts.actions.find(action => {
            const words = action.run[action.run.length - 1] === "--" ? action.run.slice(0, -1)
                : action.run;
            return words.join(" ") === run;
        });
    }

    // A row's chord, from its bind.
    function chord(run) {
        const found = bind(run);
        return found ? found.chord : [];
    }

    // Its rows that have a key stand for those actions, so a screenshot's
    // chord is found here, once.
    draws: ["niri msg action screenshot-screen", "niri msg action screenshot-window",
        "qs ipc call recording screen", "qs ipc call recording window"]
        .map(run => bind(run)).filter(found => !!found).map(found => found.does)

    // What it says beside the window it was asked of, shortened where a
    // title runs long.
    function titled(window) {
        const title = window.title || window.app_id || "untitled";
        return title.length > 48 ? `${title.slice(0, 47)}\u2026` : title;
    }

    // A window's program as its desktop entry names it, or nothing when it
    // has none, as the windows' own rows say it.
    function programOf(window) {
        const entry = window.app_id ? DesktopEntries.heuristicLookup(window.app_id) : null;
        return entry ? entry.name : "";
    }

    // Comes back here from the picker, as Escape there says.
    function back() {
        provider.launcher.into(provider.settings.name);
    }

    function shoot(window) {
        Niri.act({ ScreenshotWindow: { id: window.id, write_to_disk: true, show_pointer: false } });
    }

    // The window the keys were in. niri names a workspace's new active window
    // before it describes the window, and Niri.windows changes in place, so
    // this follows Niri.revision too: on an empty workspace, Niri.current
    // names the new window while Niri.windows doesn't hold it yet.
    readonly property var focused: {
        Niri.revision;
        return Niri.current === null ? null : Niri.windows[Niri.current] ?? null;
    }

    readonly property var stop: member({
        key: "capture:stop",
        verb: "stop",
        name: Words.name("capture:stop"),
        keywords: "end finish video",
        side: Recorder.what,
        // The vocabulary's Stop recording wears recording's mark, so this
        // one, the same act, does too.
        glyph: Theme.glyph.recording,
        hue: "action",
        chord: chord("qs ipc call recording screen"),
        run: () => Recorder.stop()
    })

    readonly property var fixed: (Recorder.recording ? [stop] : []).concat([
        member({
            key: "capture:screen",
            verb: "take",
            name: Words.name("capture:screen"),
            keywords: "screenshot picture",
            glyph: picture,
            hue: "action",
            chord: chord("niri msg action screenshot-screen"),
            later: true,
            run: () => Niri.act({ ScreenshotScreen: { write_to_disk: true, show_pointer: false } })
        })
    ], focused === null ? [] : [
        member({
            key: "capture:window",
            verb: "take",
            name: Words.name("capture:window", "", { title: titled(focused) }),
            keywords: "screenshot picture focused",
            glyph: picture,
            hue: "action",
            chord: chord("niri msg action screenshot-window"),
            later: true,
            run: () => provider.shoot(focused)
        })
    ], [
        member({
            key: "capture:region",
            verb: "pick",
            name: Words.name("capture:region"),
            keywords: "screenshot picture area part selection",
            glyph: picture,
            hue: "action",
            later: true,
            run: () => Recorder.screenshot(provider.back)
        })
    ], Recorder.recording ? [] : [
        member({
            key: "capture:record-screen",
            verb: "record",
            name: Words.name("capture:record-screen"),
            keywords: "video recording screencast",
            glyph: Theme.glyph.recording,
            hue: "action",
            chord: chord("qs ipc call recording screen"),
            later: true,
            run: () => Recorder.screen()
        })
    ], Recorder.recording || focused === null ? [] : [
        member({
            key: "capture:record-window",
            verb: "record",
            name: Words.name("capture:record-window", "", { title: titled(focused) }),
            keywords: "video recording screencast focused",
            glyph: Theme.glyph.recording,
            hue: "action",
            chord: chord("qs ipc call recording window"),
            later: true,
            run: () => Recorder.window(focused)
        })
    ], Recorder.recording ? [] : [
        member({
            key: "capture:record-region",
            verb: "pick",
            name: Words.name("capture:record-region"),
            keywords: "video recording screencast area part selection",
            glyph: Theme.glyph.recording,
            hue: "action",
            later: true,
            run: () => Recorder.region(provider.back)
        })
    ], !provider.settings.camera ? [] : [
        member({
            key: "capture:camera-picture",
            verb: "open",
            name: Words.name("capture:camera-picture"),
            keywords: "photo selfie webcam photograph",
            glyph: picture,
            hue: "action",
            later: true,
            run: () => Niri.spawn(["camera", "picture"])
        }),
        member({
            key: "capture:camera-video",
            verb: "open",
            name: Words.name("capture:camera-video"),
            keywords: "record recording webcam film movie",
            glyph: Theme.glyph.recording,
            hue: "action",
            later: true,
            run: () => Niri.spawn(["camera", "video"])
        })
    ])

    // Every other window, found only by typing its title or program, since
    // listing them all would bury the rest.
    readonly property var windows: {
        Niri.revision;
        return Object.values(Niri.windows)
            .filter(window => provider.focused === null || window.id !== provider.focused.id)
            .map(window => member({
                key: `capture:window:${window.app_id}`,
                verb: "take",
                name: titled(window),
                // Its program by the name Joel knows, never its app id.
                side: provider.programOf(window),
                glyph: picture,
                hue: "action",
                later: true,
                run: () => provider.shoot(window)
            }));
    }

    onFixedChanged: changed()

    // Where what's taken goes: niri's screenshots (modules/niri), the
    // recordings (Recorder.qml), and the camera's (modules/camera).
    readonly property string home: Quickshell.env("HOME")
    readonly property var takenIn: [`${home}/Pictures/Screenshots`, `${home}/Videos/Recordings`]
        .concat(settings.camera ? [`${home}/Pictures/Camera`, `${home}/Videos/Camera`] : [])
    // What was taken lately, newest first.
    property var taken: []

    // Stepped into, so what was taken is read again. It's read once as the
    // shell starts too, so the first step-in draws Recent where it stays
    // rather than pushing the rows below it down once find answers.
    function opened() {
        taking.running = true;
    }

    Component.onCompleted: taking.running = true

    // find's lines, a file's time and its path each, newest kept.
    function tookAll(lines) {
        taken = lines.split("\n").filter(line => line.includes("\t"))
            .map(line => ({ at: Number(line.slice(0, line.indexOf("\t"))),
                path: line.slice(line.indexOf("\t") + 1) }))
            .sort((a, b) => b.at - a.at).slice(0, 9).map(each => each.path);
        changed();
    }

    Process {
        id: taking

        command: ["find"].concat(provider.takenIn, ["-maxdepth", "1", "-type", "f", "-not",
            "-name", ".*", "-printf", "%T@\\t%p\\n"])
        stdout: StdioCollector {
            onStreamFinished: provider.tookAll(text)
        }
    }

    // Capture's Recent: a part stepped into, as a program's Recent is
    // (Applications.qml). It always stands, saying none while nothing's
    // taken, since a row that came once the first capture was read would
    // go below the rows already drawn (Steady.kept), after the screenshot's
    // own rows rather than before them.
    readonly property var recent: [entering(member({
            key: "capture:recent",
            name: Words.name("capture:recent"),
            glyph: Words.glyph("capture:recent"),
            hue: "program",
            count: taken.length === 0 ? String((Words.of("capture:recent").said ?? {}).none ?? "")
                : String(taken.length),
            ask: "which?",
            empty: Words.of("capture:recent").empty
        }), () => provider.taken.map(path => provider.takenRow(path)))]

    // One thing taken: Return opens it in its program, the picture viewer
    // or mpv, and a picture is shown in the drawer. After Open come what
    // other providers give it (`fileActs(path)`), as GIMP gives Edit in
    // GIMP, then Copy the Path.
    function takenRow(path) {
        const key = "verb:path:file:open";
        const act = thing({
            key: `${key}:${path}`,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            run: () => Niri.spawn(["xdg-open", path])
        });
        const given = (launcher?.sources ?? [])
            .filter(other => other !== provider && typeof other.fileActs === "function")
            .reduce((acts, other) => acts.concat(other.fileActs(path)), []);
        const row = Files.file(path, thing, [act].concat(given));
        if (/\.(png|jpe?g|webp)$/i.test(path))
            row.drawer = { title: row.name, picture: path };
        return row;
    }

    // A row's words, its name's and those beside it, made ready once.
    property var said: ({})

    function matched(row, words) {
        const name = Fuzzy.match(row.prepared, words, false);
        if (name.score >= 0 || !row.keywords)
            return name;
        const key = row.key + row.keywords;
        const prepared = said[key] ?? (said[key] = Fuzzy.prepare(`${row.name} ${row.keywords}`));
        const words_ = Fuzzy.match(prepared, words, false);
        // A keyword's match counts for less than the name's.
        return words_.score < 0 ? words_ : { score: words_.score / 2, grade: words_.grade };
    }

    function search(typed) {
        const words = Fuzzy.words(typed);
        const found = [];
        for (const row of fixed.concat(windows, recent)) {
            const m = matched(row, words);
            if (m.score >= 0)
                found.push({ score: m.score, grade: m.grade, row: row });
        }
        let finest = 3;
        for (const f of found)
            finest = Math.min(finest, f.grade);
        const kept = found.filter(f => f.grade === finest).sort((a, b) => b.score - a.score);
        return { rows: kept.map(f => f.row), grade: finest };
    }

    // What it captures, then what it took lately (collection-order.md).
    function everything() {
        return fixed.concat(recent);
    }

    function counted() {
        return Recorder.recording ? "recording" : "";
    }
}
