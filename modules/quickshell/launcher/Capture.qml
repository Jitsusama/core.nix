// Capture, a collection, and Print's: a picture or a video of the screen, a
// window, a region, or from the camera, so one key means capturing anything
// and the rest is chosen here, found by typing what it's of. A camera glyph
// says a picture comes of it and the recording glyph a video; the words say
// what of. The whole screen comes first, so Print then Return takes it,
// and Print twice always does. While anything records, stopping it comes
// first instead, so the gesture that started it stops it. Print over the
// launcher showing anything else takes the screen with the launcher in it
// and leaves it as it was (`pressedOver`). Each row shows its own chord
// where it has one, to learn by use and never to need.
//
// Typing a window's title finds it on any workspace, since niri takes a
// window's picture wherever it is. A region is picked by the keys
// (Picker.qml), and - there comes back here. The camera's rows open its
// viewfinder (modules/camera), and are left out where there's no camera.
// Each waits for the launcher to be gone, so it's never in the picture.
// Whether a camera is open, and each camera's parts, are the cameras' own
// (Cameras.qml). After its own rows comes its Recent: what was taken
// lately, screenshots, recordings and the camera's pictures and videos in
// one list, newest first, nine at most, each the file it is wherever it
// shows (Files.qml), a picture shown in the drawer, from the one reader of
// those folders (Taken.qml), read as the launcher opens and never
// otherwise (VKCF3Z and Y2YBTS, 2026-10-29). Typed into, Recent finds
// among everything taken, not only the nine: by its name, then a
// screenshot by the words in it, where screen-text reads them. At the top
// its matches are one row, Return stepping into Recent with the search
// kept, so what was taken keeps its one home, rather than a second list of
// the same kind of thing beside it.
//
// It holds nothing another program lodges: editing the newest screenshot
// and looking through them all are GIMP's and the picture viewer's own, in
// their programs (VKCF3Z, 2026-10-29), and a picture taken holds what each
// program that edits it gives (`fileActs`), as Edit in GIMP.
import Quickshell
import Quickshell.Io
import qs
import QtQuick
import "../FileFacts.js" as FileFacts
import "../Fuzzy.js" as Fuzzy
import "Captured.js" as Captured
import "HeldWindows.js" as HeldWindows
import "Recent.js" as Recent
import "Order.js" as Order

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

    // Comes back here from the picker, as - there goes up a level.
    function back() {
        provider.launcher.into(provider.settings.name);
    }

    function shoot(window) {
        Niri.act({ ScreenshotWindow: { id: window.id, write_to_disk: true, show_pointer: false } });
    }

    // The window the keys were in, and every other, as the windows daemon
    // says them (modules/windows), heard only while Capture's rows are
    // wanted: each event niri sent once made the seven fixed rows again
    // and walked every window, shut too (2026-11-10). Shut, it hears
    // nothing and keeps what it last made.
    readonly property var windowsDaemon: daemon

    Daemon {
        id: daemon

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/windows.sock`
        kind: ["window", "desktop"]
        live: provider.wanted
        paced: provider.launcher
        answersWhole: true
        onHeard: (changed, gone) => provider.hear(changed, gone)
    }

    // The window the keys were in, as the desktop record says it: its id,
    // title, app id and program, or null.
    property var focused: null

    // The window This Window and Record This Window both take. Until this
    // watch's first answer, or with the daemon down, the desktop record is
    // the last opening's, and a window picked as the launcher closed moved
    // the keys unheard; so niri's own word holds until then, or a quick
    // Print then Return shot the window left under its old title. Only
    // where the keys are is read of niri, so its other events remake no
    // row here.
    readonly property var here: daemon.answeredThisWatch ? focused
        : Niri.current === null ? null : Niri.windows[Niri.current] ?? null

    // Takes what the daemon said: the window the keys are in, when that
    // reads otherwise, and each window's row whose program or title
    // changed, so a whole answer as the launcher opens makes nothing anew
    // that reads the same. How many it heard of is counted (Provider.qml's
    // tally), for the check that a change is heard only of what it moved.
    function hear(changed, gone) {
        provider.tally.looked += changed.length + gone.length;
        let moved = false;
        for (const key of gone) {
            const id = key.slice("window:".length);
            if (id in provider.keptWindows) {
                delete provider.keptWindows[id];
                moved = true;
            }
        }
        for (const key of changed) {
            const record = daemon.held[key];
            if (key === "desktop") {
                const window = record.window ?? null;
                const was = provider.focused;
                if (window === null ? was !== null : was === null || !HeldWindows.same(window, was)) {
                    provider.focused = window;
                    moved = true;
                }
                continue;
            }
            const from = `${record.app_id}\n${provider.titled(record)}\n${record.entry ?? ""}`;
            const before = provider.keptWindows[record.id];
            if (before && before.from === from)
                continue;
            provider.keptWindows[record.id] = { from: from, row: provider.takingRow(record) };
            moved = true;
        }
        if (moved)
            provider.windows = Object.keys(provider.keptWindows)
                .filter(id => provider.focused === null || Number(id) !== provider.focused.id)
                .map(id => provider.keptWindows[id].row);
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

    function shootScreen() {
        Niri.act({ ScreenshotScreen: { write_to_disk: true, show_pointer: false } });
    }

    // The shortest chord that opens Capture, Print rather than Super+P.
    // Pressed here it takes the whole screen (`pressedOver`), so that row
    // shows it: a row's chord is the key that does it from where Joel
    // stands, and Shift+Print is longer here (VKCF3Z, 2026-11-03).
    readonly property var opening: (Shortcuts.layers.find(layer => layer.name === "desktop")
        ?.binds ?? [])
        .filter(found => found.run.join(" ") === `launcher-say collection ${settings.name}`)
        .map(found => found.chord)
        .reduce((shortest, one) => shortest === null || one.length < shortest.length ? one
            : shortest, null)

    readonly property var wholeScreen: member({
        key: "capture:screen",
        verb: "take",
        name: Words.name("capture:screen"),
        keywords: "screenshot picture",
        glyph: picture,
        hue: "action",
        chord: opening ?? chord("niri msg action screenshot-screen"),
        later: true,
        run: () => provider.shootScreen()
    })

    // Print or Super+P, pressed while the launcher shows, never changes
    // what's on the screen first (VKCF3Z, Joel's yes of 2026-11-03). Here
    // in Capture it's pressed twice, so it takes the work underneath, as
    // Return on the whole screen does, the launcher gone first. Over
    // anything else it takes the screen as it is, the launcher in it, and
    // leaves the launcher as it was, the keys on the same row: Joel pressed
    // it to capture what the launcher showed and found it gone. niri's
    // notice says it was taken, kept out of the next picture (config.kdl).
    function pressedOver(inside) {
        if (inside)
            provider.launcher.run(provider.wholeScreen, false, false);
        else
            provider.shootScreen();
        return true;
    }

    readonly property var fixed: (Recorder.recording ? [stop] : []).concat([
        wholeScreen
    ], here === null ? [] : [
        member({
            key: "capture:window",
            verb: "take",
            name: Words.name("capture:window", "", { title: titled(here) }),
            keywords: "screenshot picture focused",
            glyph: picture,
            hue: "action",
            chord: chord("niri msg action screenshot-window"),
            later: true,
            run: () => provider.shoot(here)
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
    ], Recorder.recording || here === null ? [] : [
        member({
            key: "capture:record-window",
            verb: "record",
            name: Words.name("capture:record-window", "", { title: titled(here) }),
            keywords: "video recording screencast focused",
            glyph: Theme.glyph.recording,
            hue: "action",
            chord: chord("qs ipc call recording window"),
            later: true,
            // The window its title names, as niri describes it, since the
            // recorder reads its size.
            run: () => Recorder.window(Niri.windows[provider.here.id] ?? null)
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
            keywords: "picture selfie webcam photograph",
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
    // listing them all would bury the rest, by id as niri numbers them. A
    // window's row is kept by its id while its program and title read the
    // same. A plain object, so keeping one notifies nothing.
    readonly property var keptWindows: ({})
    property var windows: []

    // The row that takes a picture of another window, from its record.
    function takingRow(window) {
        const id = window.id;
        return member({
            key: `capture:window:${window.app_id}`,
            verb: "take",
            name: provider.titled(window),
            // Its program by the name Joel knows, never its app id, so
            // one with no entry says none.
            side: window.entry ? window.program : "",
            glyph: provider.picture,
            hue: "action",
            later: true,
            run: () => provider.shoot({ id: id })
        });
    }

    onFixedChanged: changed()

    // Where what's taken goes: niri's screenshots (modules/niri), the
    // recordings (Recorder.qml), and the camera's (modules/camera).
    readonly property string home: Quickshell.env("HOME")
    readonly property var takenIn: [`${home}/Pictures/Screenshots`, `${home}/Videos/Recordings`]
        .concat(settings.camera ? [`${home}/Pictures/Camera`, `${home}/Videos/Camera`] : [])
    // What was taken lately, newest first, nine at most, from the one
    // reader of these folders (Taken.qml): read as the shell starts, so
    // the first step-in draws Recent where it stays, and as the launcher
    // opens, which is asked here.
    readonly property var taken: Recent.listed([Taken.files
        .filter(file => provider.takenIn.includes(file.folder))
        .map(file => ({ key: `file:${file.path}`, at: file.at, row: file.path }))], [])
        .map(one => one.row)

    onTakenChanged: changed()

    // When each thing taken was, by its path, so its row says it. Qt's
    // engine has no Object.fromEntries.
    readonly property var takenAt: Taken.files.reduce((at, file) => {
        at[file.path] = file.at;
        return at;
    }, {})
    // When is said as a file's row says it (FileFacts.js), in the
    // launcher's words and the locale's days, as everywhere (Y2YBTS,
    // 2026-11-03).
    readonly property var factWords: FileFacts.wordsOf(id => Words.of(id),
        (at, format) => Qt.formatDate(at, format))

    Component.onCompleted: {
        for (const folder of provider.takenIn)
            Taken.want(folder);
    }

    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (provider.launcher.open)
                Taken.read();
        }
    }

    // Capture's Recent: a part stepped into, as a program's Recent is
    // (Applications.qml). It always stands, saying none while nothing's
    // taken, since a row that came once the first capture was read would
    // go below the rows already drawn (Steady.kept), after the screenshot's
    // own rows rather than before them. With nothing typed it holds the
    // newest nine, laid out as every Recent is (Recent.js); typed into, it
    // finds among everything taken (`finds`).
    readonly property var stands: Recent.said(taken.map(path => path.slice(path.lastIndexOf("/")
        + 1)), false, Words.of("capture:recent").said)
    readonly property var recent: [Object.assign(entering(member({
            key: "capture:recent",
            name: Words.name("capture:recent"),
            glyph: Words.glyph("capture:recent"),
            hue: "program",
            // How many at its right, where a row's figure stands (VKCF3Z,
            // 2026-11-05: one place for each kind of fact), and their names
            // in its drawer, as every Recent says (VKCF3Z, dafc2199).
            side: stands.side,
            heldNames: stands.held,
            ask: "which?",
            empty: Words.of("capture:recent").empty
        }), () => provider.taken.map(path => provider.takenRow(path, ""))), {
            finds: letters => provider.found(letters, true)
        })]

    // Everything taken that what's typed finds, not only the nine Recent
    // shows: each file whose name matches, newest first, then each
    // screenshot whose words hold every word typed, the line that holds
    // them below it, as a source that reads them says (`takenByText`,
    // modules/screen-text). A screenshot found both ways is listed once,
    // by its name. What's read comes a moment later, when the source says
    // it found more. Inside Recent (`looking`), below them all come the
    // few pictures that look like what's typed, as a source that sees them
    // says (`takenBySight`, modules/meaning), each saying only that it was
    // found by its meaning; never at the top, where Recent gathered by a
    // guess would stand above what typing found.
    function found(letters, looking) {
        if (Fuzzy.words(letters).length === 0)
            return [];
        const byName = Taken.named(letters, provider.takenIn, provider.most);
        const rows = byName.map(path => provider.takenRow(path, ""));
        const listed = new Set(byName);
        const readers = (launcher?.sources ?? [])
            .filter(other => other !== provider && typeof other.takenByText === "function");
        for (const reader of readers)
            for (const record of reader.takenByText(letters) ?? []) {
                if (listed.has(record.path))
                    continue;
                listed.add(record.path);
                if (rows.length < provider.most * 2)
                    rows.push(provider.takenRow(record.path, record.line));
            }
        const seers = !looking ? [] : (launcher?.sources ?? [])
            .filter(other => other !== provider && typeof other.takenBySight === "function");
        for (const seer of seers)
            for (const path of seer.takenBySight(letters, listed) ?? []) {
                listed.add(path);
                rows.push(provider.takenRow(path, Words.field.byMeaning ?? ""));
            }
        return rows;
    }

    // How many files Recent finds by name, and as many again by their
    // words: more than a screen holds are never seen, and each is a row
    // made on every key.
    readonly property int most: 50

    // Recent's matches gathered at the top as one row, though Capture's
    // own rows are searched only inside it, since what was taken is
    // Capture's alone: Return steps into Recent with what's typed kept.
    function gathers(letters) {
        return { into: provider.recent[0], rows: provider.found(letters) };
    }

    // One thing taken: Return opens it in its program, the picture viewer
    // or mpv, and a picture is shown in the drawer. After Open come what
    // other providers give it (`fileActs(path)`), as GIMP gives Edit in
    // GIMP, then Copy the Path. One found by its words says below it the
    // line that holds them (`line`). Each says when it was taken at its
    // right, and its folder only where it's not Screenshots; a picture's
    // folder is said once, in its drawer's eyebrow, with what it is
    // (VKCF3Z, 2026-11-08).
    function takenRow(path, line) {
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
        const at = provider.takenAt[path];
        const folder = Files.within(path);
        const shots = Files.shown(`${provider.home}/Pictures/Screenshots`);
        row.side = Recent.takenSide(at === undefined ? ""
            : FileFacts.when(at, Date.now(), provider.factWords), folder, shots);
        // A picture's mark or a video's, as files wear them, rather than a
        // file's (VKCF3Z, 2026-11-08).
        const mark = Captured.mark(path);
        if (mark !== "")
            row.glyph = Theme.glyph[mark];
        row.drawer = () => provider.takenDrawer(path, row.name);
        if (line)
            row.note = line;
        return row;
    }

    // The drawer under one thing taken (Captured.js): where and what it is,
    // the picture, when it was taken and its size, as Taken read them, then
    // its dimensions or length once file-preview has said, which is asked
    // a frame later, never while the drawer's binding reads, since that
    // would change what it reads.
    function takenDrawer(path, name) {
        const read = Taken.files.find(file => file.path === path) ?? { path };
        const shown = provider.previews[path];
        if (shown === undefined && provider.canPreview && !provider.previewAsked[path]) {
            provider.previewAsked[path] = true;
            Qt.callLater(() => provider.preview(path));
        }
        return Captured.drawer({
            path,
            name,
            where: Files.within(path),
            at: read.at,
            size: read.size
        }, shown ?? null, {
            now: Date.now(),
            named: id => Words.name(id === "size" ? "file-chooser:size" : `capture:${id}`),
            said: Words.of("capture:recent").said ?? {},
            days: provider.factWords.days
        });
    }

    // What file-preview said of each thing taken, by its path (null where
    // it couldn't), and which were asked; the file it's asked of now, and
    // the next wanted while it answers.
    readonly property bool canPreview: (settings.preview ?? []).length > 0
    property var previews: ({})
    property var previewAsked: ({})
    property string previewNext: ""

    // Only the newest wanted waits, so one it replaces is no longer asked,
    // and its drawer asks again when it's next shown rather than never.
    function preview(path) {
        if (previewing.running) {
            if (provider.previewNext !== "" && provider.previewNext !== path)
                delete provider.previewAsked[provider.previewNext];
            provider.previewNext = path;
            return;
        }
        previewing.path = path;
        previewing.command = settings.preview.concat(["show", path]);
        previewing.running = true;
    }

    function previewed(path, text) {
        let shown = null;
        try {
            const parsed = JSON.parse(text);
            shown = typeof parsed === "object" && parsed !== null && !parsed.error
                ? parsed : null;
        } catch (error) {
            shown = null;
        }
        provider.previews = Object.assign({}, provider.previews, { [path]: shown });
    }

    Process {
        id: previewing

        property string path: ""

        stdout: StdioCollector {
            onStreamFinished: provider.previewed(previewing.path, text)
        }
        // The next is asked once this one has ended, its answer read.
        onRunningChanged: {
            if (running || provider.previewNext === "")
                return;
            const next = provider.previewNext;
            provider.previewNext = "";
            provider.preview(next);
        }
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
        const kept = Order.sorted(found.filter(f => f.grade === finest),
            (a, b) => b.score - a.score);
        return { rows: kept.map(f => f.row), grade: finest };
    }

    // What it captures, then what it took lately (collection-order.md).
    function everything() {
        return fixed.concat(recent);
    }

    // Its rows' keys for a pick (Launcher.qml's knowing), with those of the
    // rows that take a window read from niri, no row made: unwanted, they
    // stand as last made, or not at all, and a pick would forget them.
    function keysKnown() {
        const windows = Niri.current === null ? []
            : [{ key: "capture:window" }, { key: "capture:record-window" }];
        return everything().concat(windows, Object.values(Niri.windows)
            .map(window => ({ key: `capture:window:${window.app_id}` })));
    }

    function counted() {
        return Recorder.recording ? "recording" : "";
    }
}
