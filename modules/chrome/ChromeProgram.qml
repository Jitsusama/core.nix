// Chrome's own acts in Programs › Chrome (programActs, which
// Applications.qml asks every provider that has it), each found from the
// top by its name too (foundActs), and each there only while it can act.
// Open a Repository on GitHub is there while a git repository under ~/src
// has a remote on GitHub, and steps into them, the one whose git last
// wrote first, each the GitHub page it opens, by its owner and name,
// saying how long ago, found by typing as any step is (Repositories.qml).
// Each opens the remote's own page, origin's first, rather than through
// gh browse, which opens whichever browser gh is set to and may ask
// GitHub which repository it is. Open the Copied Link comes after it,
// since it comes and goes: it's there only while the clipboard holds one
// web address and nothing else, said beside it (Copied.qml). Either opens
// its page in a new window.
//
// Its Recent holds the files Chrome downloaded (`recentOf`, which
// Applications.qml asks every provider that has it), each still where
// Chrome saved it, as chrome-recent-sites lists them from its history,
// which the programs daemon runs and holds under the key `downloads`
// names, read as Chrome is stepped into, once each time the launcher
// opens, never on a timer; the shell starts no process for it. The
// daemon holds the lines as printed, and ChromeDownloads.js makes them
// into files here, a stopgap until the daemon serves them as ready rows
// on the launcher's ready-row record. Each is the one row every surface shows of a
// file (Files.qml), keyed by its path, so the list programs share never
// lists it twice, dated by when it finished, so Recent orders it among
// the sites by when, newest first (Recent.js), and opened as the desktop
// opens a file. Beside it are its size and when, as the file chooser says
// them (FileFacts.js), and its drawer, as every file's, says where it is
// then its kind, shows a picture, and says where it came from. Open the
// Latest Download comes last, there only while the last reading found
// one, its name beside it, and found from the top by its name then too.
// Until the first reading since the shell started has answered, it
// stands in its place saying an ellipsis, as Recent does while it waits,
// so it never joins the end of a page already drawn, after Quit; a
// reading that finds none takes it away.
pragma ComponentBehavior: Bound

import qs
import Quickshell
import QtQuick
import "../Copied.js" as Copying
import "../FileFacts.js" as FileFacts
import "Ago.js" as Ago
import "ChromeDownloads.js" as Downloads
import "Repositories.js" as Repos

Provider {
    id: provider

    // The desktop entry Chrome's program row is made from.
    readonly property string program: "google-chrome"
    // What opens a page in a new window, given its address after it.
    readonly property var command: settings.command ?? ["google-chrome-stable", "--new-window"]

    // The one web address the clipboard holds, or "".
    readonly property string link: Copied.link
    // The repositories with a page on GitHub, as the last reads found them,
    // the one whose git last wrote first.
    readonly property var pages: Repos.onGitHubAll(Repositories.listed, Repositories.remotes)
    readonly property bool onGitHub: pages.length > 0
    // The files Chrome downloaded, as the last reading listed them, each
    // { path, size, at, from }, and the newest of them, or null.
    property var downloads: []
    readonly property var latest: Downloads.latest(downloads)
    // Whether any reading has answered since the shell started, and so
    // whether Open the Latest Download waits in its place for one.
    property bool downloadsRead: false
    readonly property bool latestWaits: !downloadsRead && downloadsKey !== ""
    // The programs daemon's key for its downloads, or "" where none lists
    // them.
    readonly property string downloadsKey: settings.downloads ?? ""
    // So Programs › Chrome, held while any comes or goes, draws it.
    onLinkChanged: changed()
    onOnGitHubChanged: changed()
    onLatestChanged: changed()

    // Found from the top by their names, each only while it's listed.
    foundActs: (onGitHub ? [{ program: program, key: "verb:chrome:repository" }] : [])
        .concat(link !== "" ? [{ program: program, key: "verb:chrome:link" }] : [])
        .concat(latest !== null ? [{ program: program, key: "verb:chrome:download" }] : [])

    // What a download's row says, as the file chooser says a file's size
    // and when (FileFacts.js), in the launcher's words.
    readonly property var fileWords: FileFacts.wordsOf(id => Words.of(id),
        (at, format) => Qt.formatDate(at, format))

    // Whether Chrome's own step is held, from Programs or from the top,
    // and whether its downloads were read since the launcher opened.
    readonly property bool steppedIn: (launcher?.open ?? false)
        && (launcher?.held ?? []).some(step => step.row?.key === `program:${provider.program}`)
    property bool lookedSince: false

    onSteppedInChanged: {
        if (steppedIn && downloadsKey !== "")
            lookedSince = true;
    }

    // The daemon's downloads, from the first step in an opening.
    Listed {
        id: listing

        key: provider.downloadsKey
        wanted: provider.lookedSince

        onHeard: {
            provider.downloads = Downloads.listed(listing.lines);
            provider.downloadsRead = true;
            provider.changed();
        }
    }

    Component.onCompleted: {
        Repositories.want(provider.settings, true);
        Repositories.opened(provider.launcher?.open ?? false);
    }

    // What's read arrives while the pages may be shown.
    Connections {
        target: Repositories

        function onChanged() {
            provider.changed();
        }
    }

    Connections {
        target: provider.launcher

        // The next opening reads the downloads again, as they may have
        // changed since.
        function onOpenChanged() {
            Repositories.opened(provider.launcher.open);
            if (!provider.launcher.open)
                provider.lookedSince = false;
        }
    }

    function programActs(asked) {
        if (asked !== program)
            return [];
        const downloaded = latest !== null || latestWaits;
        return (onGitHub ? [repositories()] : []).concat(link !== "" ? [copied()] : [],
            downloaded ? [newest()] : []);
    }

    function recentOf(asked) {
        if (asked !== program)
            return [];
        const now = Date.now();
        return downloads.map(one => downloaded(one, now));
    }

    // Open, as the desktop opens the file, Return's on its row.
    function opens(one) {
        const key = "verb:path:file:open";
        return thing({
            key: `${key}:${one.path}`,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            run: () => Niri.spawn(Downloads.opening(one))
        });
    }

    // A download as its Recent's row: the file's own, its size and when
    // beside it, and its drawer where it is then its kind, by its ending as
    // a file nothing has read says it, a picture previewed as it is, and
    // the site it came from.
    function downloaded(one, now) {
        const row = Files.file(one.path, thing, [opens(one)]);
        const from = Downloads.cameFrom(one.from);
        const drawer = Object.assign(FileFacts.placed(Files.within(one.path),
            FileFacts.kindByName(row.name, fileWords), fileWords), {
            title: row.name,
            facts: from === "" ? [] : [{ label: Words.name("chrome:download:from"),
                value: from }]
        });
        const preview = FileFacts.pictured(one.path);
        if (preview)
            drawer.preview = preview;
        return Object.assign(row, {
            // Its size, and when where Chrome kept it, as a file whose
            // place said both is said (FileFacts.js).
            side: FileFacts.sideElsewhere({ size: one.size, modified: one.at }, now, fileWords),
            at: one.at,
            drawer: drawer
        });
    }

    // Open the Latest Download, saying its name, or an ellipsis while the
    // first reading hasn't answered.
    function newest() {
        const key = "verb:chrome:download";
        return thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            side: latest === null ? "\u2026" : Files.named(latest.path),
            // What's newest when it runs, which a row found from the top,
            // made as the act came, may not have changed with.
            run: () => {
                if (provider.latest !== null)
                    Niri.spawn(Downloads.opening(provider.latest));
            }
        });
    }

    // Open a Repository on GitHub, saying how many there are, and, stepped
    // into once a read finds none left, that there are none.
    function repositories() {
        const key = "verb:chrome:repository";
        return thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            side: String(pages.length),
            ask: Words.of(key).ask ?? "",
            empty: (Words.of(key).empty ?? "").replace("{folder}",
                Files.shown(Repositories.folder)),
            nested: true,
            targets: () => {
                const now = Date.now();
                return provider.pages.map(one => provider.page(one.page, one.at, now));
            }
        });
    }

    // A repository's page as the site it is, keyed by its address as a
    // site is wherever it shows: Return opens it, and inside, Open, then
    // Copy the Link.
    function page(github, at, now) {
        const opens = thing({
            key: `verb:recent:site:open:${github.address}`,
            name: Words.name("verb:recent:site:open"),
            verb: Words.verb("verb:recent:site:open"),
            glyph: Words.glyph("verb:recent:site:open"),
            hue: "action",
            run: () => provider.open(github.address)
        });
        const copies = thing({
            key: `verb:recent:site:link:${github.address}`,
            name: Words.name("verb:recent:site:link"),
            verb: Words.verb("verb:recent:site:link"),
            glyph: Words.glyph("verb:recent:site:link"),
            hue: "action",
            copy: github.address,
            copies: github.address,
            run: () => Quickshell.clipboardText = github.address
        });
        return thing({
            key: `site:${github.address}`,
            what: "site",
            component: "site",
            verb: opens.verb,
            name: github.name,
            glyph: Words.glyph("recent:site"),
            hue: "program",
            side: Ago.ago(at, now),
            run: opens.run,
            verbs: () => [opens, copies]
        });
    }

    // Open the Copied Link, saying the address.
    function copied() {
        const key = "verb:chrome:link";
        return thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            side: Copying.linkSaid(link),
            // What's copied when it runs, which a row found from the top,
            // made as the act came, may not have changed with.
            run: () => {
                if (Copied.link !== "")
                    provider.open(Copied.link);
            }
        });
    }

    function open(address) {
        Niri.spawn(command.concat([address]));
    }
}
