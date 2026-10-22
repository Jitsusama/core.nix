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
//
// It answers whether a camera is open, too: "camera", "webcam" or "who has
// the camera" says which programs read it, each a row by the camera it
// reads, from the margin's camera piece (pieces/Camera.qml), so the fact
// has one home. A machine whose cameras say nothing of it gets no answer.
import qs
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    what: "capture"

    collection: ({
        noun: "captures",
        title: "Capture",
        words: ["screenshot", "screen shot", "record", "recording", "video", "photo", "camera", "webcam", "print", "grab"],
        glyph: glyph(0xf0100),
        ask: "of what?"
    })

    function glyph(code) {
        return String.fromCodePoint(code);
    }

    readonly property string picture: glyph(0xf0100)

    // A row's chord, from the desktop's binds, by what it runs. A niri
    // action's command ends its options with --, which says nothing here.
    function chord(run) {
        const bind = Shortcuts.actions.find(action => {
            const words = action.run[action.run.length - 1] === "--" ? action.run.slice(0, -1)
                : action.run;
            return words.join(" ") === run;
        });
        return bind ? bind.chord : [];
    }

    // What it says beside the window it was asked of, shortened where a
    // title runs long.
    function titled(window) {
        const title = window.title || window.app_id || "untitled";
        return title.length > 48 ? `${title.slice(0, 47)}\u2026` : title;
    }

    // Comes back here from the picker, as Escape there says.
    function back() {
        provider.launcher.into(provider.settings.name);
    }

    function shoot(window) {
        Niri.act({ ScreenshotWindow: { id: window.id, write_to_disk: true, show_pointer: false } });
    }

    // The window the keys were in.
    readonly property var focused: Niri.current === null ? null : Niri.windows[Niri.current] ?? null

    readonly property var stop: member({
        key: "capture:stop",
        verb: "stop",
        name: "Stop recording",
        keywords: "end finish video",
        side: Recorder.what,
        glyph: glyph(0xf04db),
        hue: "action",
        chord: chord("qs ipc call recording screen"),
        run: () => Recorder.stop()
    })

    readonly property var fixed: (Recorder.recording ? [stop] : []).concat([
        member({
            key: "capture:screen",
            verb: "take",
            name: "The whole screen",
            keywords: "screenshot picture display monitor",
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
            name: `This window \u00b7 ${titled(focused)}`,
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
            verb: "pick one",
            name: "A region",
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
            name: "Record the whole screen",
            keywords: "video recording screencast display monitor",
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
            name: `Record this window \u00b7 ${titled(focused)}`,
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
            verb: "pick one",
            name: "Record a region",
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
            name: "A picture from the camera",
            keywords: "photo selfie webcam photograph",
            glyph: picture,
            hue: "action",
            later: true,
            run: () => Niri.spawn(["camera", "picture"])
        }),
        member({
            key: "capture:camera-video",
            verb: "open",
            name: "A video from the camera",
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
                keywords: `${window.app_id ?? ""} window screenshot picture`,
                side: window.app_id ?? "",
                glyph: picture,
                hue: "action",
                later: true,
                run: () => provider.shoot(window)
            }));
    }

    onFixedChanged: changed()

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
        for (const row of fixed.concat(windows)) {
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

    function everything() {
        return fixed.slice();
    }

    function counted() {
        return Recorder.recording ? "recording" : "";
    }

    // The words it answers whole, and by their start (Provider.qml).
    questions: ["camera", "webcam", "camera in use", "who has the camera"]

    function answer(typed) {
        const text = typed.trim().toLowerCase().replace(/\?$/, "");
        if (!/^(camera|cameras|webcam|camera in use|is the camera on|who has the camera)$/.test(text))
            return null;
        const fact = Facts.all.find(fact => fact.entry.name === "camera");
        const camera = fact ? fact.piece : null;
        if (!camera || !camera.present)
            return null;
        // Quickshell's JavaScript has no flatMap.
        const rows = camera.reading.reduce((all, seen) => all.concat(seen.programs.map(program => copyRow({
            name: program,
            side: `has ${seen.name} open`,
            glyph: Theme.glyph.camera
        }))), []);
        return {
            headline: camera.active ? `in use by ${camera.programs.join(", ")}` : "not in use",
            rows: rows,
            note: "from the cameras' own records"
        };
    }
}
