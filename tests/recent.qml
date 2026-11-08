// What Recent holds, run in Qt's own JavaScript engine, the one the shell
// runs it in. The `recent-means-one-thing` check copies
// modules/quickshell/launcher/Recent.js and Recents.js beside this file and
// runs `qml -platform offscreen recent.qml`. Each case is a promise of the
// one meaning Joel can hold in his head: the last nine things he did with
// it, newest first, each once, never what's open or shown already. The last
// are how a source's lines are read for it, and when each thing was used.
import QtQuick
import "Recent.js" as Recent
import "Recents.js" as Lines

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

    // A source's things, "key@at" each, or a bare key for one with no time.
    function kept(said) {
        return said.split(" ").filter(one => one !== "").map(one => {
            const [key, at] = one.split("@");
            return { key: key, at: at === undefined ? undefined : Number(at), row: { name: key } };
        });
    }

    function read(listed) {
        return listed.map(one => one.row.name).join(" ");
    }

    function check(failures, what, got, want) {
        if (got !== want)
            failures.push(`${what}: got ${got}, wanted ${want}`);
    }

    function run() {
        const failures = [];
        // [what, its sources, the keys shown already, what Recent lists]
        const lists = [
            ["one source is newest first", ["a@3 b@2 c@1"], "", "a b c"],
            ["two sources merge by time", ["a@5 c@1", "b@3 d@0"], "", "a b c d"],
            ["a thing is listed once, where it's newest", ["a@1 b@2", "a@5"], "", "a b"],
            ["what's open isn't listed", ["a@3 b@2 c@1"], "b", "a c"],
            ["a source with no times keeps its order after the dated", ["x y", "a@2 b@1"], "",
                "a b x y"],
            ["and its things are listed once too", ["x y x"], "", "x y"],
            ["things done at once keep their sources' order", ["a@1", "b@1"], "", "a b"],
            ["nothing kept is nothing", [], "", ""],
            ["an empty source changes nothing", ["", "a@1"], "", "a"]
        ];
        for (const [what, sources, shown, want] of lists) {
            const got = Recent.listed(sources.map(kept), shown.split(" ").filter(Boolean));
            check(failures, what, read(got), want);
        }

        const many = kept("a@12 b@11 c@10 d@9 e@8 f@7 g@6 h@5 i@4 j@3 k@2");
        check(failures, "nine at most, the newest", read(Recent.listed([many], [])),
            "a b c d e f g h i");
        check(failures, "nine after what's open is left out",
            read(Recent.listed([many], ["a", "c"])), "b d e f g h i j k");
        check(failures, "or as many as asked", read(Recent.listed([many], [], 2)), "a b");

        const words = { none: "none" };
        check(failures, "its row says how many, and their names",
            JSON.stringify(Recent.said(["Wikipedia", "Kayak"], false, words)),
            JSON.stringify({ side: "2", note: "Wikipedia, Kayak" }));
        const nine = ["Wikipedia", "Kayak", "Maps", "Mail", "Docs", "News", "Weather",
            "Photos", "Music"];
        check(failures, "as many names whole as fit, never how many more",
            Recent.said(nine, false, words, 30).note, "Wikipedia, Kayak, Maps, Mail");
        check(failures, "every name where they all fit",
            Recent.said(["Wikipedia", "Kayak", "Maps"], false, words, 22).note,
            "Wikipedia, Kayak, Maps");
        check(failures, "the first always named, though it alone is longer",
            Recent.said(nine, false, words, 5).note, "Wikipedia");
        check(failures, "it says none while it holds nothing",
            JSON.stringify(Recent.said([], false, words)),
            JSON.stringify({ side: "none", note: "" }));
        check(failures, "and waits while a source hasn't answered",
            JSON.stringify(Recent.said([], true, words)),
            JSON.stringify({ side: "\u2026", note: "" }));
        check(failures, "what one source already gave is said while another is asked",
            Recent.said(["a", "b"], true, words).side, "2");

        // A thing taken says when, its folder only where it's not the
        // usual one, which its drawer's eyebrow says instead (VKCF3Z,
        // 2026-11-08: a folder the same on every row says nothing).
        const shots = "~/Pictures/Screenshots";
        check(failures, "a screenshot says when it was taken",
            Recent.takenSide("5 min ago", shots, shots), "5 min ago");
        check(failures, "one from another folder still names it",
            Recent.takenSide("5 min ago", "~/Pictures/Camera", shots),
            "5 min ago \u00b7 ~/Pictures/Camera");
        check(failures, "and one whose time isn't known says its folder",
            Recent.takenSide("", shots, shots), shots);
        const eyebrow = Recent.takenEyebrow(shots, "screenshot", { in: "in {folder}" });
        check(failures, "its drawer says its folder and what it is", eyebrow.eyebrow,
            "in ~/Pictures/Screenshots \u00b7 screenshot");
        check(failures, "short of room, what it is goes before its folder",
            eyebrow.eyebrowItems.map(item => `${item.said}:${item.keep}`).join(", "),
            "in ~/Pictures/Screenshots:2, screenshot:1");
        check(failures, "and the folder is set as written", eyebrow.eyebrowNames, true);
        check(failures, "or its folder alone where it's no kind of its own",
            Recent.takenEyebrow(shots, "", { in: "in {folder}" }).eyebrow,
            "in ~/Pictures/Screenshots");

        // A dated source's line ends in a tab and seconds since 1970.
        const at = line => JSON.stringify(Lines.dated(line, true));
        check(failures, "a dated line's time is kept apart, in milliseconds",
            at("~/notes.md\t1700000000"), JSON.stringify({ line: "~/notes.md", at: 1700000000000 }));
        check(failures, "a line with its time left empty is undated",
            at("~/notes.md\t"), JSON.stringify({ line: "~/notes.md" }));
        check(failures, "and so is one whose time isn't digits",
            at("https://a.ca\tA Site\tsoon"), JSON.stringify({ line: "https://a.ca\tA Site" }));
        check(failures, "a line with no tab is undated, and whole",
            at("~/notes.md"), JSON.stringify({ line: "~/notes.md" }));
        check(failures, "an undated source's line is whole, tabs and all",
            JSON.stringify(Lines.dated("a\tb\t17", false)), JSON.stringify({ line: "a\tb\t17" }));

        // What recent-files prints a line: a program, a path and when.
        check(failures, "a file opened lately is its line's middle",
            Lines.openedPath("zathura\t/a.pdf\t1700000000"), "/a.pdf");
        check(failures, "whole though it holds a tab",
            Lines.openedPath("zathura\t/tab\there.pdf\t"), "/tab\there.pdf");
        check(failures, "the rest of a line with no time", Lines.openedPath("zathura\t/a.pdf"),
            "/a.pdf");
        check(failures, "and nothing from a line with no tab", Lines.openedPath("/a.pdf"), "");

        // The list many programs share: a program, a tab and a path, its
        // time kept apart already, newest first.
        const appOf = program => ({ zathura: "org.pwmt.zathura", imv: "imv" })[program] ?? "";
        const shared = ["zathura\t/b.pdf", "imv\t/a.png", "zathura\t/a.pdf",
            "zathura\t/b.pdf", "gone\t/c.txt", "zathura\t/tab\there.pdf"];
        check(failures, "each program's paths are its own, newest first, each once",
            JSON.stringify(Lines.sharedFrom(shared, appOf, 9)), JSON.stringify({
                "org.pwmt.zathura": ["/b.pdf", "/a.pdf", "/tab\there.pdf"], imv: ["/a.png"] }));
        check(failures, "at most as many as asked a program",
            JSON.stringify(Lines.sharedFrom(shared, appOf, 1)["org.pwmt.zathura"]),
            JSON.stringify(["/b.pdf"]));
        const times = { "zathura\t/b.pdf": 9000, "zathura\t/a.pdf": 4000 };
        check(failures, "each path says when it was used, and one undated says nothing",
            JSON.stringify(Lines.sharedAtFrom(shared, appOf, times)),
            JSON.stringify({ "org.pwmt.zathura": { "/b.pdf": 9000, "/a.pdf": 4000 } }));

        for (const failure of failures)
            console.warn(failure);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
