// What Recent holds, run in Qt's own JavaScript engine, the one the shell
// runs it in. The `recent-means-one-thing` check copies
// modules/quickshell/launcher/Recent.js beside this file and runs
// `qml -platform offscreen recent.qml`. Each case is a promise of the one
// meaning Joel can hold in his head: the last nine things he did with it,
// newest first, each once, never what's open or shown already.
import QtQuick
import "Recent.js" as Recent

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
        check(failures, "its row says how many, and the first one's name",
            JSON.stringify(Recent.said(2, "a", false, words)),
            JSON.stringify({ side: "2", note: "a" }));
        check(failures, "it says none while it holds nothing",
            JSON.stringify(Recent.said(0, undefined, false, words)),
            JSON.stringify({ side: "none", note: "" }));
        check(failures, "and waits while a source hasn't answered",
            JSON.stringify(Recent.said(0, undefined, true, words)),
            JSON.stringify({ side: "\u2026", note: "" }));
        check(failures, "what one source already gave is said while another is asked",
            Recent.said(2, "a", true, words).side, "2");

        for (const failure of failures)
            console.warn(failure);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
