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
pragma ComponentBehavior: Bound

import qs
import Quickshell
import QtQuick
import "../Copied.js" as Copying
import "Ago.js" as Ago
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
    // So Programs › Chrome, held while either comes or goes, draws it.
    onLinkChanged: changed()
    onOnGitHubChanged: changed()

    // Found from the top by their names, each only while it's listed.
    foundActs: (onGitHub ? [{ program: program, key: "verb:chrome:repository" }] : [])
        .concat(link !== "" ? [{ program: program, key: "verb:chrome:link" }] : [])

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

        function onOpenChanged() {
            Repositories.opened(provider.launcher.open);
        }
    }

    function programActs(asked) {
        if (asked !== program)
            return [];
        return (onGitHub ? [repositories()] : []).concat(link !== "" ? [copied()] : []);
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
