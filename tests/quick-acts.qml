// What decides whether a program's act on a copy or a repository shows,
// and what it opens, run in Qt's own JavaScript engine, the one the shell
// runs it in: what the clipboard's history holds of what the clipboard
// holds read back as its types and text, its lines kept, and any other
// record passed over (Copied.js); a picture only among
// picture types, PNG asked for first, its type said and saved by its
// own name; one web address alone, not one among words, two, or another
// kind of link; a file's path from the root, home or a file URL, with a
// line and a column after it as compilers and grep give them, and nothing
// for a relative path, a folder's or two lines; a markdown file, and a
// document zathura reads, by its extension; the repositories dated from
// what stat says of each .git, the one written last first, a repository's files the same, and each one's
// page on GitHub from its remotes, origin's first, however git names the
// host, those listed that have one, and a file's folder said within its
// repository (Repositories.js); Neovim opened at a line and column, in
// the kitty used last or a new one (NeovimOpening.js); what
// each Neovim says it holds, read whole lines alone, every file once from
// the Neovim looked at last and the file used last in it, said modified
// and by its tab, hidden or shown, what only Neovim says in a file's
// drawer, its buffer and when it was last in a window where it said, the
// command going to its buffer, and the program's drawer's counts
// (Neovim.js); and Chrome's downloads read from the lines
// chrome-recent-sites prints, those without a path or a size left out and
// one with no time undated, the latest the first, said by the host it came
// from and opened as the desktop opens a file (ChromeDownloads.js). The
// `the-launcher-acts-only-on-what-it-can` check copies the six beside
// this file and runs `qml quick-acts.qml`.
import QtQuick
import "ChromeDownloads.js" as Downloads
import "Copied.js" as Copying
import "Repositories.js" as Repos
import "NeovimOpening.js" as Opening
import "Neovim.js" as Neovim

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

    // What each Neovim says it holds, as Neovim.js reads it.
    function neovims(check) {
        const held = (files, tabs) => JSON.stringify({ pid: 1, folder: "/r", tab: 1, tabs: tabs,
            current: 1, files: files });
        const file = (buffer, path, modified, tabs, at) => ({ buffer, path, modified, tabs, at });
        const lines = [
            `@5\t41:7\t/run/a\t${held([file(1, "/r/a.rs", false, [1], 100),
                file(2, "/r/b.rs", true, [2], 200), file(3, "/r/c.rs", false, [], 50),
                file(4, "/r/d.rs", false, [1, 2], 10)], 2)}`,
            `@6\t41:8\t/run/b\t${held([file(1, "/r/a.rs", true, [1], 300),
                file(2, "/q/e.md", false, [1], 20)], 1)}`,
            "@7\t41:9\t/run/c\tnot json",
            `9\t41:9\t/run/c\t${held([], 1)}`,
            "@8\t41:9",
            ""
        ];
        const heard = Neovim.heard(lines.join("\n"));
        check("only whole lines read", heard.map(one => [one.window, one.go, one.server]),
            [[5, "41:7", "/run/a"], [6, "41:8", "/run/b"]]);
        // Window 5 was looked at after window 6.
        const focus = { 5: 20, 6: 10 };
        const open = Neovim.open(heard, window => focus[window] ?? 0);
        check("each file once, the Neovim looked at last first, the file used last first",
            open.map(one => `${one.path} ${one.buffer} ${one.windows} ${one.neovim.window}`),
            ["/r/b.rs 2 1 5", "/r/a.rs 1 2 5", "/r/c.rs 3 1 5", "/r/d.rs 4 1 5",
                "/q/e.md 2 1 6"]);
        const said = { modified: "modified", tab: "tab {index}", tabs: "tabs {indices}",
            shown: "shown", hidden: "hidden", windows: "in {count} windows",
            inWindow: "{count} in 1 window", inWindows: "{count} in {windows} windows" };
        check("modified first, then its tab, then how many windows hold it",
            open.map(one => Neovim.side(one, said)),
            ["modified \u00b7 tab 2", "modified \u00b7 tab 1 \u00b7 in 2 windows", "hidden",
                "tabs 1, 2", "shown"]);
        // Neovim says when in seconds, the launcher in milliseconds; a
        // buffer never in a window says no time.
        const when = at => `at ${at}`;
        check("when a file was last in a window, then its buffer, an identifier, last",
            Neovim.plateFacts(open[0], when),
            [{ id: "nvim:used", value: "at 200000" }, { id: "nvim:buffer", value: "2" }]);
        check("no time where Neovim never had it in a window",
            Neovim.plateFacts(Object.assign({}, open[0], { at: 0 }), when),
            [{ id: "nvim:buffer", value: "2" }]);
        check("going to its buffer through its kitty window",
            Neovim.going("/bin/open-now", open[0]),
            ["/bin/open-now", "neovim", "go", "41:7", "/run/a", "2"]);
        check("what the drawer says Neovim holds",
            Neovim.held(heard, open, said),
            [{ id: "nvim:files", value: "5 in 2 windows" }, { id: "nvim:modified", value: "2" }]);
        check("one window, nothing modified",
            Neovim.held(heard.slice(1), open.slice(4), said),
            [{ id: "nvim:files", value: "1 in 1 window" }]);
        check("nothing open, nothing said", Neovim.held([], [], said), []);
    }

    function run() {
        const failures = [];
        let asked = 0;
        const check = (what, said, meant) => {
            asked += 1;
            if (JSON.stringify(said) !== JSON.stringify(meant))
                failures.push(
                    `${what}: said ${JSON.stringify(said)}, meant ${JSON.stringify(meant)}`);
        };
        const home = "/home/joel";

        // The history's record of what the clipboard holds, read back.
        const now = (types, text) => ({ key: "now:clipboard", kind: "now", now: { types, text } });
        check("a text copy", Copying.held(now(["text/plain", "UTF8_STRING"], "one\ntwo")),
            { types: ["text/plain", "UTF8_STRING"], text: "one\ntwo" });
        check("a picture copy", Copying.held(now(["image/png", "image/jpeg"], "")),
            { types: ["image/png", "image/jpeg"], text: "" });
        check("a copy with nothing to read", Copying.held(now([], "")),
            { types: [], text: "" });
        check("a tab inside the text kept", Copying.held(now(["text/plain"], "a\tb")),
            { types: ["text/plain"], text: "a\tb" });
        check("no record yet", Copying.held(undefined), null);
        check("a record of something else", Copying.held({ key: "summary:copies" }), null);
        check("what it can't read read as nothing",
            Copying.held({ now: { types: ["a", 1], text: 2 } }),
            { types: ["a"], text: "" });

        // [types, the type asked for]
        const pictures = [
            [["image/jpeg", "image/png", "text/html"], "image/png"],
            [["text/html", "image/webp"], "image/webp"],
            [["text/plain", "UTF8_STRING"], ""],
            [[], ""]
        ];
        for (const [types, meant] of pictures)
            check(`the picture among ${types.join(" ")}`, Copying.picture(types), meant);
        check("PNG said", Copying.pictureSaid("image/png"), "PNG");
        check("an unknown picture said", Copying.pictureSaid("image/avif"), "AVIF");
        check("JPEG saved", Copying.extension("image/jpeg"), "jpg");
        check("an unknown picture saved", Copying.extension("image/x-portable-pixmap"),
            "xportablepixmap");

        // [copied, the address it is]
        const links = [
            ["https://github.com/Jitsusama/core.nix/pull/12",
                "https://github.com/Jitsusama/core.nix/pull/12"],
            ["  http://localhost:8080/  \n", "http://localhost:8080/"],
            ["https://192.168.1.4/admin?x=1#top", "https://192.168.1.4/admin?x=1#top"],
            ["https://[fe80::1]:443/", "https://[fe80::1]:443/"],
            ["see https://example.com", ""],
            ["https://a.example https://b.example", ""],
            ["https://a.example\nhttps://b.example", ""],
            ["mailto:joel@example.com", ""],
            ["ftp://example.com/file", ""],
            ["example.com", ""],
            ["https://", ""],
            ["https://-bad-.example", ""]
        ];
        for (const [copied, meant] of links)
            check(`the link in ${JSON.stringify(copied)}`, Copying.link(copied), meant);
        check("a link said", Copying.linkSaid("https://github.com/Jitsusama/"),
            "github.com/Jitsusama");

        // [copied, the place it names]
        const at = (path, line, column) => {
            const cut = path.lastIndexOf("/");
            return { path: path, folder: cut === 0 ? "/" : path.slice(0, cut),
                name: path.slice(cut + 1), line: line, column: column };
        };
        const places = [
            ["/etc/hosts", at("/etc/hosts", 0, 0)],
            ["~/src/core.nix/flake.nix", at(`${home}/src/core.nix/flake.nix`, 0, 0)],
            ["src/main.rs:12", null],
            ["~/src/core.nix/flake.nix:120", at(`${home}/src/core.nix/flake.nix`, 120, 0)],
            ["/x/main.rs:12:5", at("/x/main.rs", 12, 5)],
            ["/x/main.rs:12:5:", at("/x/main.rs", 12, 5)],
            ["/x/main.rs:12:", at("/x/main.rs", 12, 0)],
            ["/x/a:3/main.rs:12", at("/x/a:3/main.rs", 12, 0)],
            ["/x/main.rs:0:4", at("/x/main.rs", 0, 0)],
            ["file:///home/joel/My%20Notes/plan.md", at(`${home}/My Notes/plan.md`, 0, 0)],
            ["file://localhost/etc/hosts", at("/etc/hosts", 0, 0)],
            ["file:///bad%ZZ", null],
            ["/vmlinuz", at("/vmlinuz", 0, 0)],
            ["/etc/", null],
            ["/etc/..", null],
            ["/etc/hosts\n/etc/passwd", null],
            ["main.rs", null],
            ["", null]
        ];
        for (const [copied, meant] of places)
            check(`the place in ${JSON.stringify(copied)}`, Copying.place(copied, home), meant);
        check("a place said from home", Copying.placeSaid(at(`${home}/src/a.rs`, 12, 5), home),
            "~/src/a.rs:12:5");
        check("a place said elsewhere", Copying.placeSaid(at("/etc/hosts", 0, 0), home),
            "/etc/hosts");
        check("a folder's URL, escaped", Copying.folderUrl("/home/joel/a #1/b%"),
            "file:///home/joel/a%20%231/b%25");
        check("a name's filter, its wildcards their own", Copying.nameFilter("a*b?[c].md"),
            "a[*]b[?][[]c[]].md");
        // [path, markdown]
        const kinds = [["/a/README.md", true], ["/a/notes.MARKDOWN", true], ["/a/x.mdx", false],
            ["/a/md", false]];
        for (const [path, meant] of kinds)
            check(`${path} is markdown`, Copying.markdown(path), meant);
        // [path, a document zathura reads]
        const documents = [["/a/charts.pdf", true], ["/a/SCAN.DJVU", true], ["/a/b.epub", true],
            ["/a/c.ps", true], ["/a/d.pdf.txt", false], ["/a/pdf", false], ["/a/e.png", false],
            ["/a/README.md", false]];
        for (const [path, meant] of documents)
            check(`${path} is a document`, Copying.document(path), meant);

        // The repositories, the one whose git wrote last first, two alike by
        // their paths, and what isn't a .git left out.
        check("the repositories dated", Repos.repositories([
            "1700000000\t/home/joel/src/old/.git",
            "1700000500\t/home/joel/src/new/.git",
            "1700000500\t/home/joel/src/also/.git",
            "1700000900\t/home/joel/src/notes.git",
            "garbled",
            ""
        ].join("\n")), [
            { path: "/home/joel/src/also", at: 1700000500000 },
            { path: "/home/joel/src/new", at: 1700000500000 },
            { path: "/home/joel/src/old", at: 1700000000000 }
        ]);
        check("a repository's files dated", Repos.dated(
            "1\t/r/a.md\n3\t/r/b c.rs\n2\t/r/d/e"), [
            { path: "/r/b c.rs", at: 3000 },
            { path: "/r/d/e", at: 2000 },
            { path: "/r/a.md", at: 1000 }
        ]);
        const many = Array.from({ length: 5001 },
            (_, at) => ({ path: `/r/${at}`, at: 5001 - at }));
        check("a large repository's files, only the 5,000 changed last",
            Repos.newest(many).map(one => one.path).slice(-1).concat(Repos.newest(many).length),
            ["/r/4999", 5000]);

        // Each repository's remotes, in the order its config gives them, a
        // url before any remote's heading, or under a submodule's, left out.
        const remotes = Repos.remotes([
            "/s/a/.git/config:\turl = https://stray.example/x",
            "/s/a/.git/config:[remote \"origin\"]",
            "/s/a/.git/config:\turl = git@github.com:Jitsusama/a.git",
            "/s/a/.git/config:[remote \"mirror\"]",
            "/s/a/.git/config:   url=https://gitlab.com/j/a",
            "/s/a/.git/config:[submodule \"vendor\"]",
            "/s/a/.git/config:\turl = https://github.com/v/vendor",
            "/s/b/.git/config:[remote \"upstream\"]",
            "/s/b/.git/config:\turl = https://github.com/NixOS/nixpkgs.git"
        ].join("\n"));
        check("the remotes read", remotes, {
            "/s/a": [{ name: "origin", url: "git@github.com:Jitsusama/a.git" },
                { name: "mirror", url: "https://gitlab.com/j/a" }],
            "/s/b": [{ name: "upstream", url: "https://github.com/NixOS/nixpkgs.git" }]
        });

        // [remote, its page's address, or "" for one elsewhere]
        const urls = [
            ["git@github.com:Jitsusama/core.nix.git", "https://github.com/Jitsusama/core.nix"],
            ["github.com:Jitsusama/core.nix", "https://github.com/Jitsusama/core.nix"],
            ["ssh://git@github.com/Jitsusama/core.nix", "https://github.com/Jitsusama/core.nix"],
            ["ssh://git@github.com:22/o/r.git", "https://github.com/o/r"],
            ["https://github.com/o/r.git", "https://github.com/o/r"],
            ["https://github.com/o/r/", "https://github.com/o/r"],
            ["https://joel:token@github.com/o/r", "https://github.com/o/r"],
            ["git://github.com/o/r", "https://github.com/o/r"],
            ["https://gitlab.com/o/r", ""],
            ["https://github.com/o", ""],
            ["https://github.com.evil.example/o/r", ""],
            ["/home/joel/src/r", ""]
        ];
        for (const [url, meant] of urls)
            check(`the page of ${url}`, Repos.github(url)?.address ?? "", meant);
        check("a page named by its owner and name",
            Repos.github("git@github.com:Jitsusama/core.nix.git").name, "Jitsusama/core.nix");
        check("origin's page first", Repos.onGitHub([
            { name: "upstream", url: "https://github.com/up/r" },
            { name: "origin", url: "git@github.com:me/r.git" }
        ])?.address, "https://github.com/me/r");
        check("then upstream's", Repos.onGitHub([
            { name: "mine", url: "https://github.com/mine/r" },
            { name: "upstream", url: "https://github.com/up/r" },
            { name: "origin", url: "https://gitlab.com/me/r" }
        ])?.address, "https://github.com/up/r");
        check("then the first on GitHub", Repos.onGitHub([
            { name: "work", url: "https://gitlab.com/w/r" },
            { name: "mine", url: "https://github.com/mine/r" }
        ])?.address, "https://github.com/mine/r");
        check("none on GitHub", Repos.onGitHub(remotes["/s/none"]), null);
        // Those listed with a page there, in the order they're listed.
        check("the repositories on GitHub", Repos.onGitHubAll([
            { path: "/s/b", at: 2 },
            { path: "/s/none", at: 1 },
            { path: "/s/a", at: 0 }
        ], remotes).map(one => `${one.path} ${one.at} ${one.page.name}`),
            ["/s/b 2 NixOS/nixpkgs", "/s/a 0 Jitsusama/a"]);
        check("none there", Repos.onGitHubAll([{ path: "/s/none", at: 1 }], remotes), []);
        // [a file, the folder it's said to be in within its repository]
        const within = [["/s/a/main.rs", ""], ["/s/a/docs/notes.md", "docs"],
            ["/s/a/a/b/c.md", "a/b"], ["/s/ab/x.md", "/s/ab"]];
        for (const [path, meant] of within)
            check(`${path} within /s/a`, Repos.within(path, "/s/a"), meant);

        // Neovim at a line, and a column after it.
        check("Neovim given a file", Opening.neovim("/r/a.rs", 0, 0), ["nvim", "/r/a.rs"]);
        check("at its line", Opening.neovim("/r/a.rs", 12, 0), ["nvim", "+12", "/r/a.rs"]);
        check("at its column", Opening.neovim("/r/a.rs", 12, 5),
            ["nvim", "+call cursor(12, 5)", "/r/a.rs"]);
        // The kitty used last, of the windows niri lists, and none.
        const windows = [
            { app_id: "kitty", pid: 40, focus_timestamp: { secs: 10, nanos: 5 } },
            { app_id: "google-chrome", pid: 50, focus_timestamp: { secs: 99, nanos: 0 } },
            { app_id: "kitty", pid: 41, focus_timestamp: { secs: 10, nanos: 9 } },
            { app_id: "kitty", pid: 42 }
        ];
        check("the kitty used last", Opening.kitty(windows), 41);
        check("no kitty", Opening.kitty(windows.filter(one => one.app_id !== "kitty")), 0);
        check("in the kitty running", Opening.inKitty(41, "/r", ["nvim", "/r/a.rs"]),
            ["kitty", "@", "--to", "unix:@kitty-41", "launch", "--type=os-window", "--cwd", "/r",
                "nvim", "/r/a.rs"]);
        check("in a new kitty", Opening.inKitty(0, "/r", ["nvim", "/r/a.rs"]),
            ["kitty", "--directory", "/r", "nvim", "/r/a.rs"]);

        neovims(check);

        // Chrome's downloads as chrome-recent-sites prints them: a path,
        // its size, when in seconds and where from, each line its own.
        const downloads = Downloads.listed([
            "/h/Downloads/report.pdf\t2048\t1800000100\thttps://docs.example.com/q3?x=1",
            "/h/Downloads/undated.zip\t10\t\t",
            "/h/Downloads/bare.txt\t3",
            "\t5\t1\t",
            "/h/Downloads/no-size\t\t1\t",
            "/h/Downloads/odd\tbig\t1\t"
        ]);
        check("each with a path and a size, when in milliseconds", downloads, [
            { path: "/h/Downloads/report.pdf", size: 2048, at: 1800000100000,
                from: "https://docs.example.com/q3?x=1" },
            { path: "/h/Downloads/undated.zip", size: 10, at: null, from: "" },
            { path: "/h/Downloads/bare.txt", size: 3, at: null, from: "" }
        ]);
        check("the latest the first printed", Downloads.latest(downloads)?.path,
            "/h/Downloads/report.pdf");
        check("none while there's none", Downloads.latest([]), null);
        check("where it came from by its host",
            Downloads.cameFrom("https://docs.example.com/q3?x=1"), "docs.example.com");
        check("a host with no path", Downloads.cameFrom("http://files.example.org"),
            "files.example.org");
        check("nothing from nowhere", Downloads.cameFrom(""), "");
        check("nothing from what isn't an address", Downloads.cameFrom("about:blank"), "");
        check("opened as the desktop opens a file", Downloads.opening(downloads[0]),
            ["xdg-open", "/h/Downloads/report.pdf"]);

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        console.log(`${asked - failures.length} of ${asked} held`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
