// kitty's own, in Programs › kitty, given as the program's acts
// (programActs, which Applications.qml asks every provider that has it),
// after its New Window and New Window Below and before its Recent:
//
// - New Window in a Repository, while ~/src holds any: a list of them,
//   those zoxide knows first in its order, each opening a kitty window
//   there on Return, as a folder in kitty's Recent does.
// - SSH to a Machine, while ~/.ssh/config names any: a list of them, one
//   connected now first, then each by when zsh's history last ran ssh to
//   it, then as written, each opening a kitty window running kitty's ssh
//   kitten on Return.
// - Tabs, while a kitty window is open: every tab of every kitty OS window
//   niri shows, the window used last first and its tabs as they were used,
//   each saying how it stands, what runs in it and where. Return goes to
//   one, its OS window through niri and then the tab inside it through
//   kitty's remote control; inside are Go To, its panes where it has more
//   than one, each the same, and Close It, which keeps the launcher and
//   leaves the tab where it was, said closed, until it opens again.
//
// A window opens through the kitty behind the window used last, as a new
// OS window of it, or as a new kitty while none runs. Everything is read
// as kitty is stepped into, once each time the launcher opens, and never
// otherwise (VKCF3Z): kitty's own state through open-now, which asks every
// kitty at once, fd for the repositories, zoxide for their order, grep for
// the ssh lines of the history, and ~/.ssh/config itself. What kitty says
// is read in Kitty.js, and the rest in KittyOpens.js.
pragma ComponentBehavior: Bound

import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "Kitty.js" as Kitty
import "KittyOpens.js" as Opens

Provider {
    id: provider

    what: "tab"

    // Whether what kitty's step shows was asked for since the launcher
    // opened; what each said is kept until it says it again.
    property bool looked: false
    property var kitties: []
    property string found: ""
    property string visited: ""
    property string history: ""
    property string config: ""
    // The tabs and panes closed since the launcher opened, by key.
    property var closed: []

    readonly property var tools: ({
        kitten: provider.settings.kitten ?? "kitten",
        kitty: provider.settings.kitty ?? "kitty"
    })

    // The launcher closed, so the next step into kitty reads it all again.
    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (!provider.launcher.open) {
                provider.looked = false;
                provider.closed = [];
            }
        }
    }

    function look() {
        if (looked)
            return;
        looked = true;
        // ~/.ssh/config is read as its path is given, below.
        for (const reader of [listing, finding, ranking, searching]) {
            if ((reader.command[0] ?? "") !== "" && !reader.running)
                reader.running = true;
        }
    }

    // Each answer draws kitty's step again, once a frame however many come
    // together.
    function heard() {
        Qt.callLater(provider.redraw);
    }

    function redraw() {
        provider.changed();
    }

    Process {
        id: listing

        command: [provider.settings.openNow ?? "", "kitty"]
        stdout: StdioCollector {
            id: listed

            onStreamFinished: {
                provider.kitties = Kitty.heard(listed.text);
                provider.heard();
            }
        }
    }

    Process {
        id: finding

        command: [provider.settings.fd ?? "", "--hidden", "--type", "d", "--max-depth", "4",
            "--prune", "^\\.git$", provider.settings.src ?? ""]
        stdout: StdioCollector {
            id: repositories

            onStreamFinished: {
                provider.found = repositories.text;
                provider.heard();
            }
        }
    }

    Process {
        id: ranking

        command: (provider.settings.zoxide ?? "") === "" ? [] : [provider.settings.zoxide, "query",
            "--list", "--sort-by", "last-accessed"]
        stdout: StdioCollector {
            id: ranked

            onStreamFinished: {
                provider.visited = ranked.text;
                provider.heard();
            }
        }
    }

    // Only the history's ssh lines, of however many it keeps.
    Process {
        id: searching

        command: (provider.settings.history ?? "") === "" ? [] : [provider.settings.grep, "-a",
            "-E", "^: [0-9]+:[0-9]+;\\s*(ssh|kitten ssh|kitty \\+kitten ssh)\\s",
            provider.settings.history]
        stdout: StdioCollector {
            id: searched

            onStreamFinished: {
                provider.history = searched.text;
                provider.heard();
            }
        }
    }

    FileView {
        id: sshConfig

        path: provider.looked ? provider.settings.sshConfig ?? "" : ""
        printErrors: false
        onLoaded: {
            provider.config = sshConfig.text();
            provider.heard();
        }
        // One that isn't there names no machine; letting go of the path as
        // the launcher closes keeps what it said until it's read again.
        onLoadFailed: {
            if (sshConfig.path === "")
                return;
            provider.config = "";
            provider.heard();
        }
    }

    function niri() {
        return Object.values(Niri.windows);
    }

    function tabsNow() {
        return Kitty.tabs(provider.kitties, provider.niri());
    }

    function words(id) {
        return Words.of(id).said ?? {};
    }

    // An act or a part as the vocabulary says it, keyed by what it's done
    // to, with its : command.
    function said(id, key, fields) {
        return thing(Object.assign({
            key: key,
            verb: Words.verb(id),
            name: Words.name(id),
            glyph: Words.glyph(id),
            hue: "action"
        }, Words.ending(id), fields));
    }

    function programActs(program) {
        if (program !== "kitty")
            return [];
        provider.look();
        return provider.repositoryAct().concat(provider.sshAct(), provider.tabsRow());
    }

    // A new kitty window, through the kitty behind the window used last.
    function open(where) {
        Niri.spawn(Opens.opening(provider.tools, Opens.latest(provider.niri()), where));
    }

    function repositoriesNow() {
        return Opens.repositories(provider.found, provider.visited);
    }

    function repositoryAct() {
        if (provider.repositoriesNow().length === 0)
            return [];
        const id = "verb:kitty:repository";
        return [provider.said(id, id, {
            ask: Words.of(id).ask ?? "",
            nested: true,
            targets: () => provider.repositoriesNow().map(path => provider.repositoryRow(path))
        })];
    }

    // A repository as every folder is (Files.qml), opening a window there.
    function repositoryRow(path) {
        const id = "verb:path:folder:window";
        return Files.folder(path, thing, [provider.said(id, `${id}:${path}`, {
            run: () => provider.open({ folder: path })
        })]);
    }

    // What's known of when each machine was reached, for its row.
    function known() {
        const open = provider.tabsNow().reduce((all, tab) => all.concat(tab.panes), [])
            .map(pane => pane.runs.machine).filter(name => name !== "");
        return { open: open, last: Opens.connected(provider.history), time: Date.now() };
    }

    function machinesNow(known) {
        return Opens.machinesInOrder(Opens.machines(provider.config), known.last, known.open);
    }

    function sshAct() {
        if (Opens.machines(provider.config).length === 0)
            return [];
        const id = "verb:kitty:ssh";
        return [provider.said(id, id, {
            ask: Words.of(id).ask ?? "",
            nested: true,
            targets: () => {
                const now = provider.known();
                return provider.machinesNow(now).map(one => provider.machineRow(one, now));
            }
        })];
    }

    // A machine: Return opens a window running kitty's ssh kitten to it, by
    // the name ~/.ssh/config gives it; inside, that and Copy the Name.
    function machineRow(one, known) {
        const ssh = provider.said("verb:kitty:machine:ssh", `verb:kitty:machine:ssh:${one.name}`, {
            run: () => provider.open({ command: [provider.tools.kitten, "ssh", one.name] })
        });
        const copying = "verb:kitty:machine:copy";
        const copy = provider.said(copying, `${copying}:${one.name}`, {
            copy: one.name,
            copies: one.name,
            run: () => Quickshell.clipboardText = one.name
        });
        const said = provider.words("kitty:machine");
        return thing({
            thing: true,
            key: `kitty:machine:${one.name}`,
            what: "machine",
            name: one.name,
            side: Opens.machineSide(one, known, said),
            glyph: Words.glyph("kitty:machine"),
            hue: "program",
            verb: ssh.verb,
            run: ssh.run,
            verbs: () => [ssh, copy],
            drawer: () => Opens.machineDrawer(one, known, {
                said: said,
                named: id => Words.name(`kitty:machine:${id}`)
            })
        });
    }

    function tabsRow() {
        const count = provider.tabsNow().length;
        if (count === 0)
            return [];
        const id = "kitty:tabs";
        return [provider.said(id, id, {
            hue: "program",
            side: String(count),
            ask: Words.of(id).ask ?? "",
            nested: true,
            targets: () => provider.tabsNow().map(tab => provider.tabRow(tab))
        })];
    }

    // What runs in a pane wears its program's mark where it has one, ssh
    // the mark of another machine, and anything else kitty's.
    function glyphOf(pane) {
        if (pane.runs.machine !== "")
            return Words.glyph("kitty:machine");
        const own = provider.programGlyphs[pane.runs.program] !== undefined;
        return provider.programGlyph(own ? pane.runs.program : "kitty");
    }

    // Where a tab is: its place in its OS window's bar, then niri's place
    // of that window where it isn't on the workspace in front.
    function whereIs(tab) {
        const window = Niri.windows[tab.window] ?? null;
        const place = window ? provider.placeOf(window, Niri.workspaces[window.workspace_id],
            Niri.workspace(), 1) : "";
        return [Kitty.placeOf(tab, provider.words("kitty:tabs")), place]
            .filter(part => part !== "").join(" \u00b7 ");
    }

    // Goes to a tab or a pane: its OS window through niri, then itself
    // inside it through kitty, which asks niri for nothing.
    function go(one) {
        Niri.act({ FocusWindow: { id: one.window } });
        Niri.spawn(Kitty.going(provider.tools.kitten, one));
    }

    function close(one) {
        Niri.spawn(Kitty.closing(provider.tools.kitten, one));
        const panes = (one.panes ?? []).map(pane => pane.key);
        provider.closed = provider.closed.concat([one.key], panes);
        provider.changed();
    }

    // A tab's or a pane's row: Return goes to it; inside are Go To, a
    // tab's panes where it holds more than one, and Close It. One closed
    // stays dimmed where it was, holding nothing.
    function row(one, kind, where) {
        const said = provider.words("kitty:tabs");
        const pane = one.panes ? one.panes[0] : one;
        const drawer = () => Object.assign(Kitty.drawerOf(pane, {
            said: provider.words("kitty:pane"),
            home: Files.home,
            named: id => Words.name(`kitty:pane:${id}`)
        }, Date.now()), { eyebrow: where });
        const fields = {
            thing: true,
            key: one.key,
            what: kind,
            name: Kitty.nameOf(one, said),
            keywords: Kitty.keywordsOf(one, said),
            note: where,
            glyph: provider.glyphOf(pane),
            hue: "program",
            drawer: drawer
        };
        if (provider.closed.includes(one.key))
            return thing(Object.assign(fields, {
                side: Kitty.closedSide(one, said, Files.home),
                gone: true,
                verb: Words.verb(`kitty:${kind}`),
                run: () => {},
                verbs: () => []
            }));
        const goes = provider.said(`verb:kitty:${kind}:go`, `verb:kitty:${kind}:go:${one.key}`, {
            run: () => provider.go(one)
        });
        const closes = provider.said(`verb:kitty:${kind}:close`,
            `verb:kitty:${kind}:close:${one.key}`, {
                stays: true,
                run: () => provider.close(one)
            });
        const panes = kind === "tab" && one.panes.length > 1
            ? one.panes.map(each => provider.row(each, "pane", where)) : [];
        const acts = [goes].concat(panes, [closes]);
        return thing(Object.assign(fields, {
            side: Kitty.sideOf(one, said, Files.home),
            verb: goes.verb,
            run: goes.run,
            verbs: () => acts
        }));
    }

    function tabRow(tab) {
        return provider.row(tab, "tab", provider.whereIs(tab));
    }
}
