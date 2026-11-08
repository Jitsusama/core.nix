// What the launcher's search by meaning works out, run in Qt's own
// JavaScript engine, the one the shell runs it in: which things the
// meaning daemon is told and as what, how what's typed is asked, the
// rows an answer adds, and the pictures Capture's Recent adds below its
// own finds. The `meaning-adds-only-below` check copies
// modules/meaning/Meaning.js beside this file and runs
// `qml -platform offscreen meaning.qml`.
import QtQuick
import "Meaning.js" as Meaning

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    function run() {
        const failures = [];
        const expect = (name, got, want) => {
            if (JSON.stringify(got) !== JSON.stringify(want))
                failures.push(`${name}: ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
        };

        // A thing as the launcher keys it, with the words it's known by as
        // Provider.qml prepares them.
        const thing = (key, name, known, more) => Object.assign({
            key: key,
            name: name,
            keywords: known === null ? null : { text: known }
        }, more ?? {});
        const brightness = thing("action:brightness", "Brightness",
            "Brightness screen backlight");
        const kitty = thing("program:kitty", "Terminal", null);
        const window = thing("window:12", "kitty: ~/src", null, { is: "window:12" });
        const session = thing("agents:pi-3", "Session", "Session pi");
        const blank = thing("action:nothing", "", null);

        // Told: what stays, as the words it's known by, once a key.
        expect("a thing is told by the words it's known by",
            Meaning.things([brightness]),
            [{ key: "action:brightness", text: "Brightness screen backlight" }]);
        expect("one known by its name alone is told its name",
            Meaning.things([kitty]), [{ key: "program:kitty", text: "Terminal" }]);
        expect("rows that come and go are never told",
            Meaning.things([window, session, brightness]).map(told => told.key),
            ["action:brightness"]);
        expect("a key is told once", Meaning.things([kitty, kitty]).length, 1);
        expect("a thing with nothing to say is left out", Meaning.things([blank]), []);
        expect("a meaning's own row is never told again",
            Meaning.things([Object.assign({}, kitty, { byMeaning: true })]), []);
        expect("no choices tell nothing", Meaning.things(undefined), []);
        // Told again only when what's told changes: `as` stands for it all.
        expect("the same things are told as the same",
            Meaning.telling([brightness, window]).as, Meaning.telling([brightness]).as);
        expect("another thing is told as another",
            Meaning.telling([brightness, kitty]).as === Meaning.telling([brightness]).as,
            false);

        // Asked: in lower case, its spaces single, from three letters.
        expect("what's typed is asked plainly", Meaning.asked("  Dim  the Screen "),
            "dim the screen");
        expect("two letters are too few", Meaning.enough("di"), false);
        expect("three are enough", Meaning.enough("dim"), true);
        expect("spaces aren't letters", Meaning.enough("a b"), false);

        // The rows: nearest first, none shown already, none gone, a few.
        const keyed = {
            "action:brightness": brightness,
            "program:kitty": kitty,
            "window:12": window,
            "action:volume": thing("action:volume", "Volume", null),
            "action:night": thing("action:night", "Night Light", null)
        };
        const records = keys => keys.map((key, at) => ({ key: key, kind: "thing",
            near: 0.9 - at / 100 }));
        const note = "found by its meaning";
        const how = { most: 3, note: note };
        const rows = Meaning.rows(records(["action:brightness", "program:kitty"]), keyed,
            new Set(), how);
        expect("an answer's things are rows in its order", rows.map(row => row.key),
            ["action:brightness", "program:kitty"]);
        expect("each says how it was found, below everything typed found",
            rows.map(row => [row.note, row.byMeaning, row.tier]),
            [[note, true, 3], [note, true, 3]]);
        expect("the launcher's own thing is left as it was", brightness.note, undefined);
        // Its own note stays after how it was found, as when typed, and
        // gives way first (Y2YBTS, 2026-11-08).
        const placed = thing("place:screens", "Screens", null, { note: "in Machine" });
        expect("its own note follows how it was found",
            Meaning.rows(records(["place:screens"]), { "place:screens": placed }, new Set(),
                how).map(row => row.note), ["found by its meaning \u00b7 in Machine"]);
        expect("one already found by its letters isn't added again",
            Meaning.rows(records(["action:brightness", "program:kitty"]), keyed,
                new Set(["action:brightness"]), how).map(row => row.key),
            ["program:kitty"]);
        expect("a window is known by what it is",
            Meaning.rows(records(["window:12"]), keyed, new Set(["window:12"]), how),
            []);
        expect("a thing the launcher no longer holds is left out",
            Meaning.rows(records(["action:gone", "program:kitty"]), keyed, new Set(), how)
                .map(row => row.key), ["program:kitty"]);
        expect("no more than the most are added",
            Meaning.rows(records(["action:brightness", "program:kitty", "action:volume",
                "action:night"]), keyed, new Set(), how).length, 3);
        const shown = new Set(["program:kitty"]);
        Meaning.rows(records(["action:brightness"]), keyed, shown, how);
        expect("what's shown is left as it was", Array.from(shown), ["program:kitty"]);
        expect("no answer adds nothing", Meaning.rows(undefined, keyed, new Set(), how), []);

        // The pictures: nearest first, by path, none listed already, a few.
        const looks = (keys, kind) => keys.map((key, at) => ({ key: key,
            kind: kind ?? "picture", near: 0.15 - at / 100 }));
        expect("an answer's pictures are paths in its order",
            Meaning.paths(looks(["/s/chart.png", "/s/code.png"]), new Set(), 3),
            ["/s/chart.png", "/s/code.png"]);
        expect("one Recent listed by its name or words isn't added again",
            Meaning.paths(looks(["/s/chart.png", "/s/code.png"]), new Set(["/s/chart.png"]), 3),
            ["/s/code.png"]);
        expect("no more pictures than the most are added",
            Meaning.paths(looks(["/a.png", "/b.png", "/c.png", "/d.png"]), new Set(), 3).length,
            3);
        expect("a picture answered twice is added once",
            Meaning.paths(looks(["/a.png", "/a.png"]), new Set(), 3), ["/a.png"]);
        expect("a thing is never a picture",
            Meaning.paths(looks(["action:brightness"], "thing"), new Set(), 3), []);
        const listed = new Set(["/a.png"]);
        Meaning.paths(looks(["/b.png"]), listed, 3);
        expect("what's listed is left as it was", Array.from(listed), ["/a.png"]);
        expect("no answer adds no picture", Meaning.paths(undefined, new Set(), 3), []);

        for (const failure of failures)
            console.warn(failure);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
