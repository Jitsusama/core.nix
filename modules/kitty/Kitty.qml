// kitty's own, in Programs › kitty, given as the program's acts
// (programActs, which Applications.qml asks every provider that has it),
// after its New Window and New Window Below and before its Recent:
//
// - New Window in a Repository, while ~/src holds any: a list of them,
//   those zoxide knows first in its order, each opening a kitty window
//   there on Return, as a folder in kitty's Recent does.
// - SSH to a Machine, while ~/.ssh/config or ~/.ssh/known_hosts names
//   any: a list of them, most used first, by how many times zsh's history
//   ran ssh to each, then by when it last did, then as written, each
//   opening a kitty window running kitty's ssh kitten on Return. The
//   glossary's word is machine, never host, so typing host finds it as a
//   name it once had.
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
// by the programs daemon, over the one socket the shell holds open to it,
// starting no command here (Joel, 2026-11-10: the daemons are the data
// layer): as the shell starts and each time the launcher opens, kitty's
// own state through open-now, which asks every kitty at once
// (`listed:kitty:open`), the repositories under ~/src (Repositories.qml,
// which Chrome's and Neovim's acts read too), when zoxide last saw each
// folder, from its database, ~/.ssh/config and ~/.ssh/known_hosts, and
// the ssh lines of zsh's history (modules/programs/src/visited.rs and
// machines.rs). So each act is there by the time kitty's page draws, in
// its place, since a row that arrives once the page is drawn joins its
// end until a key makes the list afresh (Y2YBTS, 2026-11-08). What kitty
// says is read in Kitty.js, and the rest in KittyOpens.js, still here
// until Programs serves its own ready rows (Thu 11-12).
pragma ComponentBehavior: Bound

import qs
import Quickshell
import QtQuick
import "../Hearing.js" as Hearing
import "Kitty.js" as Kitty
import "KittyOpens.js" as Opens

Provider {
    id: provider

    what: "tab"

    // What each read said, kept until it says it again.
    property var kitties: []
    property string visited: ""
    // When zoxide last saw each folder, in milliseconds by its path.
    property var visits: ({})
    property string history: ""
    property string config: ""
    property string hosts: ""
    // The tabs and panes closed since the launcher opened, by key.
    property var closed: []

    readonly property var tools: ({
        kitten: provider.settings.kitten ?? "kitten",
        kitty: provider.settings.kitty ?? "kitty"
    })

    // The launcher's closing forgets the tabs closed meanwhile; its
    // opening reads nothing, as the daemon tells each file as it changes.
    Connections {
        target: provider.launcher

        function onOpenChanged() {
            Repositories.opened(provider.launcher.open);
            if (!provider.launcher.open)
                provider.closed = [];
        }
    }

    Component.onCompleted: provider.wantRepositories()
    // The settings name the folder the repositories are under, and may be
    // given after the provider is made.
    onSettingsChanged: provider.wantRepositories()

    // The daemon's key for each file read, by what's kept of it; none
    // where the settings name no file.
    readonly property var keys: {
        const keyed = (kind, path) => (path ?? "") === "" ? [] : [`${kind}:${path}`];
        return keyed("visited", provider.settings.zoxideData)
            .concat(keyed("machines", provider.settings.ssh), keyed("reached",
                provider.settings.history));
    }

    function wantRepositories() {
        Repositories.want({ folder: provider.settings.src ?? "" });
    }

    // What the daemon holds of each file now, kept as it tells one
    // changed: nothing is asked as the launcher opens.
    function read() {
        const held = kind => Object.values(daemon.held).find(one => one.kind === kind);
        const visited = held("visited")?.folders ?? [];
        provider.visited = visited.map(one => one.path).join("\n");
        provider.visits = visited.reduce((all, one) =>
            Object.assign(all, { [one.path]: one.at }), {});
        provider.config = held("machines")?.config ?? "";
        provider.hosts = held("machines")?.hosts ?? "";
        provider.history = (held("reached")?.lines ?? []).join("\n");
        provider.heard();
    }

    Daemon {
        id: daemon

        socket: `${Quickshell.env("XDG_RUNTIME_DIR") ?? ""}/daemons/programs.sock`
        kind: "visited"
        live: false
        // The daemon reads each file as its watch begins, and again only as
        // it's written, telling only a record that differs.
        ahead: provider.keys

        onHeard: changed => {
            if (changed.some(key => provider.keys.includes(key)))
                provider.read();
        }
    }

    // What open-now printed, as the programs daemon holds it under the
    // key the settings name, read as the shell starts and each time the
    // launcher opens.
    Listed {
        id: listing

        key: provider.settings.open ?? ""
        wanted: (provider.launcher?.open ?? false) || !listing.read

        onHeard: {
            provider.kitties = Kitty.heard(listing.lines.join("\n"));
            provider.heard();
        }
    }

    // The repositories read arrive as they're read.
    Connections {
        target: Repositories

        function onChanged() {
            provider.heard();
        }
    }

    // Each answer draws kitty's step again, once a frame however many come
    // together, timed from the earliest line that asked (Hearing.js).
    function heard() {
        Qt.callLater(Hearing.putOff(redrawn));
    }
    readonly property var redrawn: Hearing.carried(() => provider.redraw())

    function redraw() {
        provider.changed();
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
        return provider.repositoryAct().concat(provider.sshAct(), provider.tabsRow());
    }

    // What its drawer in Programs says it holds (Applications.qml asks
    // every provider's programFacts), from what was read ahead, never a
    // read of its own.
    function programFacts(program) {
        if (program !== "kitty")
            return [];
        return Kitty.held(provider.tabsNow(), provider.repositoriesNow().length,
            provider.everyMachine().length, provider.words("kitty:tabs"))
            .map(fact => ({ label: Words.name(fact.id), value: fact.value }));
    }

    // A new kitty window, through the kitty behind the window used last.
    function open(where) {
        Niri.spawn(Opens.opening(provider.tools, Opens.latest(provider.niri()), where));
    }

    function repositoriesNow() {
        return Opens.repositories(Repositories.listed.map(one => one.path).join("\n"),
            provider.visited);
    }

    function repositoryAct() {
        const count = provider.repositoriesNow().length;
        if (count === 0)
            return [];
        const id = "verb:kitty:repository";
        return [provider.said(id, id, {
            side: String(count),
            ask: Words.of(id).ask ?? "",
            nested: true,
            targets: () => provider.repositoriesNow().map(path => provider.repositoryRow(path))
        })];
    }

    // A repository as every folder is (Files.qml), opening a window there,
    // its side when zoxide last saw it, and the folder holding it where
    // that isn't ~/src, which every row would say again (VKCF3Z), or that
    // it was never opened.
    function repositoryRow(path) {
        const id = "verb:path:folder:window";
        const side = Opens.repositorySide(path, { src: provider.settings.src ?? "",
            home: Files.home }, provider.visits, { now: Date.now(),
            never: provider.words("verb:kitty:repository").never });
        const sided = fields => thing(fields.what === "folder"
            ? Object.assign(fields, { side: side }) : fields);
        return Files.folder(path, sided, [provider.said(id, `${id}:${path}`, {
            run: () => provider.open({ folder: path })
        })]);
    }

    // What's known of when and how often each machine was reached, and
    // which an ssh session is open to now, for its row.
    function reaching() {
        const open = provider.tabsNow().reduce((all, tab) => all.concat(tab.panes), [])
            .map(pane => pane.runs.machine).filter(name => name !== "");
        return { open: open, reached: Opens.connected(provider.history), time: Date.now() };
    }

    function everyMachine() {
        return Opens.machines(provider.config, provider.hosts);
    }

    function machinesNow(state) {
        return Opens.machinesInOrder(provider.everyMachine(), state.reached);
    }

    function sshAct() {
        const count = provider.everyMachine().length;
        if (count === 0)
            return [];
        const id = "verb:kitty:ssh";
        return [provider.said(id, id, {
            side: String(count),
            ask: Words.of(id).ask ?? "",
            nested: true,
            targets: () => {
                const now = provider.reaching();
                return provider.machinesNow(now).map(one => provider.machineRow(one, now));
            }
        })];
    }

    // A machine: Return opens a window running kitty's ssh kitten to it, by
    // the name its file gives it; inside, that and Copy the Name. One
    // known_hosts names on two ports is two machines, keyed by each.
    function machineRow(one, state) {
        const at = one.port === "" ? one.name : `${one.name}:${one.port}`;
        const ssh = provider.said("verb:kitty:machine:ssh", `verb:kitty:machine:ssh:${at}`, {
            run: () => provider.open({ command: Opens.sshing(provider.tools.kitten, one) })
        });
        const copying = "verb:kitty:machine:copy";
        const copy = provider.said(copying, `${copying}:${at}`, {
            copy: one.name,
            copies: one.name,
            run: () => Quickshell.clipboardText = one.name
        });
        const said = provider.words("kitty:machine");
        return thing({
            thing: true,
            key: `kitty:machine:${at}`,
            what: "machine",
            name: one.name,
            side: Opens.machineSide(one, state, said),
            glyph: Words.glyph("kitty:machine"),
            hue: "program",
            verb: ssh.verb,
            run: ssh.run,
            verbs: () => [ssh, copy],
            drawer: () => Opens.machineDrawer(one, state, {
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
        // One the table doesn't know wears the theme's program mark, so
        // that answer means it has none of its own.
        const own = provider.programGlyph(pane.runs.program);
        const unknown = provider.launcher?.marks?.program ?? "";
        return own !== unknown ? own : provider.programGlyph("kitty");
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
