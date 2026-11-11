// What swayimg's page in the launcher offers to look through, run in Qt's
// own JavaScript engine, the one the shell runs it in. The
// `swayimg-leads-with-a-gallery` check copies modules/swayimg/Swayimg.js
// into launcher/ beside it, the FileFacts.js and When.js it says a count
// and a time by, and the launcher's words.json, and runs
// `qml -platform offscreen swayimg-galleries.qml`. Each case is one the
// page holds to: a gallery of each folder the pictures in its Recent are
// in, newest first, each once, opening on the newest of Recent's pictures
// still there, or on the folder where none is, and not offered once it's
// counted holding none; the screenshots' gallery leading while it holds
// any, else the newest folder's, else nothing; what a gallery's row says
// beside it; what each gallery's drawer shows; and what the drawer says
// of the screenshots.
import QtQuick
import "FileFacts.js" as FileFacts
import "launcher/Swayimg.js" as Swayimg

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

    property var failures: []

    function expect(what, got, wanted) {
        if (JSON.stringify(got) !== JSON.stringify(wanted))
            failures.push(`${what}: got ${JSON.stringify(got)}, wanted ${JSON.stringify(wanted)}`);
    }

    // Words with their no-break spaces written as underscores, to read.
    function kept(words) {
        return words.replace(/_/g, "\u00a0");
    }

    // Each thing as words.json says it.
    function launcherWords() {
        const request = new XMLHttpRequest();
        request.open("GET", Qt.resolvedUrl("words.json"), false);
        request.send();
        const things = {};
        for (const thing of JSON.parse(request.responseText).things)
            things[thing.id] = thing;
        return id => things[id] ?? {};
    }

    function run() {
        const of = launcherWords();
        const pictures = of("swayimg:screenshots").said ?? {};

        // What swayimg's Recent lists, newest first: two pictures from
        // Downloads with one from home between them, one kept straight in
        // the Nix store, one that isn't a whole path, and a screenshot.
        const recent = ["/home/joel/Downloads/cat.png", "/home/joel/tidepool.jpg",
            "/home/joel/Downloads/dog.webp", "/nix/store/abc-wallpaper.png", "relative.png",
            "/home/joel/Pictures/Screenshots/shot.png"];
        const shots = "/home/joel/Pictures/Screenshots";

        expect("each folder once, newest first, its pictures newest first",
            Swayimg.folders(recent, []), [
                { folder: "/home/joel/Downloads",
                    pictures: ["/home/joel/Downloads/cat.png", "/home/joel/Downloads/dog.webp"] },
                { folder: "/home/joel", pictures: ["/home/joel/tidepool.jpg"] },
                { folder: shots, pictures: [`${shots}/shot.png`] }
            ]);
        expect("the screenshots' folder isn't listed while its own gallery is",
            Swayimg.folders(recent, [shots]).map(one => one.folder),
            ["/home/joel/Downloads", "/home/joel"]);
        expect("a picture at the root is in /", Swayimg.folders(["/a.png"], []),
            [{ folder: "/", pictures: ["/a.png"] }]);
        expect("nothing in Recent is no folder", Swayimg.folders([], []), []);

        expect("a folder is asked of the programs daemon by its path",
            Swayimg.folderKey("/home/joel/Downloads"), "folder:/home/joel/Downloads");

        // Each file as the daemon says it: when it last changed, in
        // seconds, its size, then its path.
        const changed = (seconds, path) => `${seconds}\t2048\t${path}`;
        const within = ["/home/joel/Downloads", "/home/joel", "/home/joel/Empty"];
        const listed = Swayimg.read([changed("1794070100.25", "/home/joel/Downloads/dog.webp"),
            changed("1794070900", "/home/joel/Downloads/notes.txt"),
            changed("1794070300.5", "/home/joel/Downloads/DRAWING.SVG"),
            changed("1794070200", "/home/joel/Downloads/raw.jpeg"),
            changed("1794060000", "/home/joel/tidepool.jpg"),
            changed("1794070000", "/home/joel/Empty/readme.md"),
            changed("1794070000", "/elsewhere/stray.png")], within);
        expect("only pictures count, of only the folders read, a folder with none none",
            listed.counts, { "/home/joel/Downloads": 3, "/home/joel": 1, "/home/joel/Empty": 0 });
        expect("the pictures there are known by path", Object.keys(listed.there).sort(),
            ["/home/joel/Downloads/DRAWING.SVG", "/home/joel/Downloads/dog.webp",
                "/home/joel/Downloads/raw.jpeg", "/home/joel/tidepool.jpg"]);
        const drawing = { path: "/home/joel/Downloads/DRAWING.SVG", at: 1794070300500 };
        const pool = { path: "/home/joel/tidepool.jpg", at: 1794060000000 };
        expect("each folder's newest picture, of pictures alone", listed.newest,
            { "/home/joel/Downloads": drawing, "/home/joel": pool });

        expect("before a read, each opens on Recent's newest there, uncounted",
            Swayimg.galleries(recent, null, [shots]), [
                { folder: "/home/joel/Downloads", count: null,
                    target: "/home/joel/Downloads/cat.png", newest: null },
                { folder: "/home/joel", count: null, target: "/home/joel/tidepool.jpg",
                    newest: null }
            ]);
        expect("once read, each says how many, opening on the newest of Recent's still there",
            Swayimg.galleries(recent, listed, [shots]), [
                { folder: "/home/joel/Downloads", count: 3,
                    target: "/home/joel/Downloads/dog.webp", newest: drawing },
                { folder: "/home/joel", count: 1, target: "/home/joel/tidepool.jpg",
                    newest: pool }
            ]);
        expect("a folder whose pictures from Recent are all gone opens on its newest",
            Swayimg.galleries(["/home/joel/Downloads/gone.png"], listed, []),
            [{ folder: "/home/joel/Downloads", count: 3, target: "/home/joel/Downloads",
                newest: drawing }]);
        expect("a folder counted holding none isn't offered, nor one that's gone",
            Swayimg.galleries(["/home/joel/Empty/a.png", "/home/joel/Gone/b.png"],
                Swayimg.read([], ["/home/joel/Empty", "/home/joel/Gone"]), []), []);
        expect("a folder Recent gained since the read stands uncounted",
            Swayimg.galleries(["/home/joel/New/a.png"], listed, []),
            [{ folder: "/home/joel/New", count: null, target: "/home/joel/New/a.png",
                newest: null }]);

        expect("a gallery being counted says so", Swayimg.side(null, "~", pictures),
            "\u2026 \u00b7 ~");
        expect("one picture is one, where it is after",
            Swayimg.side(1, "~/Pictures", pictures), kept("1_picture \u00b7 ~/Pictures"));
        expect("many are many", Swayimg.side(42, "~", pictures), kept("42_pictures \u00b7 ~"));
        expect("home is nowhere further", Swayimg.side(3, "", pictures), kept("3_pictures"));
        expect("a gallery is named by its folder",
            Swayimg.named(of("verb:swayimg:folder").name, "Downloads"), "Look Through Downloads");

        const offered = Swayimg.galleries(recent, listed, [shots]);
        expect("the screenshots' gallery leads while it holds any",
            Swayimg.lead(12, offered), "screenshots");
        expect("else the newest folder's", Swayimg.lead(0, offered), "/home/joel/Downloads");
        expect("else nothing", Swayimg.lead(0, []), "");

        const gallery = ["swayimg", "--gallery", "--execute=newest first"];
        expect("a gallery opens on what it's given",
            Swayimg.opening(gallery, "/home/joel/tidepool.jpg"),
            ["swayimg", "--gallery", "--execute=newest first", "/home/joel/tidepool.jpg"]);

        expect("swayimg draws what its build reads, by its ending, in any case",
            ["a.PNG", "b.jpg", "c.svgz", "d.heic", "e.ppm", "f.txt", "g.png.part", "h"]
                .map(name => Swayimg.isPicture(name)),
            [true, true, true, true, true, false, false, false]);

        const words = FileFacts.wordsOf(of, (at, format) => Qt.formatDate(at, format));
        const now = new Date(2026, 10, 10, 16, 30).getTime();
        const newest = { path: `${shots}/shot.png`, at: now - 12 * 60_000 };
        expect("the drawer says when the newest screenshot was taken, then how many",
            Swayimg.held(newest, 214, now, words), [
                { id: "swayimg:screenshots:newest", value: kept("12_min ago") },
                { id: "swayimg:screenshots:count", value: "214" }
            ]);
        expect("and nothing while there are none", Swayimg.held(null, 0, now, words), []);

        // Each gallery's drawer, a folder's plate with its newest picture.
        const plated = { file: words, labelled: id => of(id).name };
        const lately = { path: "/home/joel/Pictures/lighthouse.png", at: now - 3 * 60_000 };
        const folder = Swayimg.plate({ where: "~", name: "Pictures", count: 3,
            newest: lately, target: "/home/joel/Pictures/tide.png", taken: false }, now, plated);
        expect("a folder's gallery says where its folder is, then its kind",
            [folder.eyebrow, folder.eyebrowNames, folder.title],
            ["in ~ \u00b7 Folder", true, "Pictures"]);
        expect("its newest picture previewed, as a file's drawer previews one", folder.preview,
            { picture: "/home/joel/Pictures/lighthouse.png", across: 0, down: 0 });
        expect("its newest's name, when it was added, and how many", folder.facts, [
            { label: "Newest", value: "lighthouse.png" },
            { label: "Added", value: kept("3_min ago") },
            { label: "Pictures", value: "3" }
        ]);
        const screenshots = Swayimg.plate({ where: "~/Pictures", name: "Screenshots",
            count: 214, newest, taken: true }, now, plated);
        expect("the screenshots' says when its newest was taken",
            [screenshots.eyebrow, screenshots.facts.map(fact => fact.label),
                screenshots.preview?.picture],
            ["in ~/Pictures \u00b7 Folder", ["Newest", "Taken", "Pictures"], `${shots}/shot.png`]);
        const unread = Swayimg.plate({ where: "~", name: "Downloads", count: null,
            newest: null, target: "/home/joel/Downloads/cat.png", taken: false }, now, plated);
        expect("before its folder is read, what it opens on, and nothing more said",
            [unread.preview?.picture, unread.facts],
            ["/home/joel/Downloads/cat.png", []]);
        expect("a picture the shell can't draw isn't previewed", Swayimg.plate({ where: "~",
            name: "Downloads", count: 3, newest: drawing, taken: false }, now, plated).preview,
            undefined);
        expect("each fact is named in the launcher's words",
            ["swayimg:screenshots:newest", "swayimg:screenshots:count"].map(id => of(id).name),
            ["Newest Screenshot", "Screenshots"]);

        if (failures.length > 0) {
            for (const failure of failures)
                console.warn(`FAIL ${failure}`);
            Qt.exit(1);
        } else {
            Qt.exit(0);
        }
    }
}
