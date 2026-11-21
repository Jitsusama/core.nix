// When Copies hides what each copy holds, run in Qt's own JavaScript
// engine, the one the shell runs it in. The
// `copies-hide-while-the-screen-is-shared` check copies launcher/Shared.js,
// launcher/Steady.js and tests/lib/Expect.js beside this file and runs it
// over casts made up here, as niri says them.
import QtQuick
import "Shared.js" as Shared
import "Steady.js" as Steady
import "Expect.js" as Expect

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
            Qt.exit(0);
        } catch (error) {
            Expect.fail(`${error}`);
        }
    }

    // A cast as niri's CastsChanged says one, by its stream.
    function cast(stream, kind, session, active) {
        return { [stream]: { stream_id: stream, session_id: session, kind: kind,
            is_active: active, target: { Output: { name: "eDP-1" } } } };
    }

    function run() {
        Expect.same("nothing cast, the copies show", Shared.hidden(true, {}, null), false);
        Expect.same("before niri says its casts, they hide", Shared.hidden(false, {}, null), true);
        Expect.same("and with no word of them at all", Shared.hidden(undefined, undefined, null),
            true);
        Expect.same("a call sharing the screen hides them",
            Shared.hidden(true, cast(4, "PipeWire", 9, true), null), true);
        Expect.same("and one starting, not yet streaming",
            Shared.hidden(true, cast(4, "PipeWire", 9, false), null), true);
        Expect.same("the shell recording one window shows them",
            Shared.hidden(true, cast(4, "PipeWire", 9, true), 9), false);
        Expect.same("but not while a call shares beside it",
            Shared.hidden(true, Object.assign(cast(4, "PipeWire", 9, true),
                cast(5, "PipeWire", 12, true)), 9), true);
        // A recording is a share made later: what it read lands in a file
        // that goes wherever the file does (5GLTPH, 2026-11-17).
        Expect.same("a screen recorder outside the portal hides them too",
            Shared.hidden(true, cast(4, "WlrScreencopy", 9, true), null), true);
        Expect.same("even one sharing the recorded window's session",
            Shared.hidden(true, cast(4, "WlrScreencopy", 9, true), 9), true);
        Expect.same("and a kind niri adds later",
            Shared.hidden(true, cast(4, "Mirror", 9, true), 9), true);
        Expect.same("even beside the shell's own recording of one window",
            Shared.hidden(true, Object.assign(cast(4, "PipeWire", 9, true),
                cast(5, "WlrScreencopy", 12, true)), 9), true);
        whyCases();
        shownCases();
        madeCases();
    }

    // What a copy's row is made with beyond what it holds: made showing, it
    // names the source that hides it, so no way back goes to a place holding
    // it once a share starts (Steady.hiddenGone, run here on it); made
    // hidden, it names none and blanks what its row would draw of the copy.
    function madeCases() {
        const copies = { hiding: false };
        const asked = [];
        const name = () => {
            asked.push("name");
            return "Hidden text";
        };
        const showing = Shared.made(false, copies, name);
        Expect.same("a row made showing names the very source that hides it",
            showing.shownBy === copies, true);
        Expect.same("and changes nothing else it draws", Object.keys(showing), ["shownBy"]);
        Expect.same("nor looks up the hidden row's name", asked, []);
        // A place holding a row made showing, as the launcher keeps one.
        const place = row => ({ held: [{ row: Object.assign({ key: "copy" }, row) }] });
        const ways = row => ({ back: [place(row)], ahead: [], tags: [], last: null });
        const kept = row => Steady.hiddenGone(ways(row)).back.length;
        const before = ways(showing);
        Expect.same("before a share, a way back still goes to it",
            Steady.hiddenGone(before).back.length, 1);
        copies.hiding = true;
        Expect.same("once a share starts, that same row lets its way back go",
            Steady.hiddenGone(before).back.length, 0);
        const hidden = Shared.made(true, copies, name);
        Expect.same("a row made hidden names no source, so a place holding it stays",
            ["shownBy" in hidden, kept(hidden)], [false, 1]);
        Expect.same("and blanks what its row would draw of the copy", hidden,
            { name: "Hidden text", note: "", reads: "", picture: "" });
    }

    // Why they hide, as the journal says it, agreeing with hidden().
    function whyCases() {
        Expect.same("before niri says its casts, the journal says so",
            Shared.why(false, {}, null), "niri hasn't said its casts yet");
        Expect.same("nothing cast, nothing to say", Shared.why(true, {}, null), "");
        Expect.same("nor for the shell's own recording alone",
            Shared.why(true, cast(4, "PipeWire", 9, true), 9), "");
        Expect.same("a share says what sees the screen, the recording left out",
            Shared.why(true, Object.assign(cast(4, "PipeWire", 9, true),
                cast(5, "WlrScreencopy", 12, true)), 9), "1 cast sees the screen (WlrScreencopy)");
        Expect.same("and counts each",
            Shared.why(true, Object.assign(cast(4, "PipeWire", 9, true),
                cast(5, "WlrScreencopy", 12, true)), null),
            "2 casts see the screen (PipeWire, WlrScreencopy)");
        Expect.same("a kind is said only as a plain word",
            Shared.why(true, cast(4, "Pipe\nWire", 9, true), null),
            "1 cast sees the screen (Pipe?Wire)");
        // And says something exactly when hidden() hides, over every case.
        const both = Object.assign(cast(4, "PipeWire", 9, true),
            cast(5, "WlrScreencopy", 12, true));
        const inputs = [[false, {}, null], [undefined, undefined, null], [true, {}, null],
            [true, cast(4, "PipeWire", 9, true), null], [true, cast(4, "PipeWire", 9, false), null],
            [true, cast(4, "PipeWire", 9, true), 9], [true, cast(4, "PipeWire", 9, true), 12],
            [true, cast(4, "WlrScreencopy", 9, true), 9], [true, cast(4, "Mirror", 9, true), 9],
            [true, both, 9], [true, both, null], [false, both, 9]];
        const disagree = inputs.filter(one => (Shared.why(one[0], one[1], one[2]) !== "")
            !== Shared.hidden(one[0], one[1], one[2]));
        Expect.same("why() speaks exactly when hidden() hides", disagree.length, 0);
    }

    // What a hidden copy's row is made of, from copies as the history
    // serves them (modules/quickshell/clipboard/src/rows.rs).
    function shownCases() {
        const came = { at: 1000, as: "came" };
        const link = { thing: "7", kind: "text", glyph: "link", side: [came],
            acts: [{ key: "verb:copy:copy" }] };
        Expect.same("a hidden link's glyph says only that it's a copy",
            Shared.shown(link).glyph, "clipboard");
        Expect.same("and its acts stay, so it can still be copied",
            JSON.stringify(Shared.shown(link).acts), JSON.stringify(link.acts));
        Expect.same("what the history served is left as it was", link.glyph, "link");
        const lines = { kind: "text", glyph: "lines", side: [{ said: "2 lines" }, came] };
        Expect.same("a hidden text's side keeps when it came, not its lines",
            JSON.stringify(Shared.shown(lines).side), JSON.stringify([came]));
        Expect.same("and the side served is left whole", lines.side.length, 2);
        const later = { kind: "text", side: [{ at: 900, as: "opened" }, came] };
        Expect.same("a timed item other than when it came is hidden too",
            JSON.stringify(Shared.shown(later).side), JSON.stringify([came]));
        const picture = { kind: "picture", glyph: "picture",
            side: [{ said: "PNG" }, { said: "1.52 MB" }, came] };
        Expect.same("a hidden picture is still a picture", Shared.shown(picture).glyph,
            "picture");
        Expect.same("but says neither its format nor its size",
            JSON.stringify(Shared.shown(picture).side), JSON.stringify([came]));
        Expect.same("a copy served with no side has none",
            JSON.stringify(Shared.shown({ kind: "text" }).side), "[]");
    }
}
