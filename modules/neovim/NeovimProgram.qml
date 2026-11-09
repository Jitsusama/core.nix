// Neovim's own acts in Programs › Neovim (programActs, which
// Applications.qml asks every provider that has it), each found from the
// top by its name too (foundActs), and each there only while it can act.
// Open a File by Name is there while a git repository is under ~/src,
// and steps into them, the one whose git last wrote first, each saying
// how long ago, and on into a repository's files, the one changed last
// first, each beside its folder in the repository, found by typing as any
// step is (Repositories.qml). Open the Copied Path comes after it, since
// it comes and goes: it's there only while the clipboard holds an
// existing file's path, said beside it, and opens it at the line, and the
// column after that, the copy gives (Copied.qml), as a compiler's or
// ripgrep's "main.rs:12:5" says them.
//
// Either opens Neovim in a new window of the kitty used last, through
// kitty's remote control, as kitty's Recent opens a folder, in the
// repository or the file's folder, or in a new kitty there while none
// runs (NeovimOpening.js).
//
// After them, what's open now: every file each Neovim in a kitty window
// has open, a file a row and each once, the Neovim looked at last first
// and in it the file used last, its side saying whether it's modified and
// which tab shows it, hidden while none does, before its folder. Return
// goes to it, its kitty window and then the buffer inside Neovim, never
// starting another; inside are Go To and Copy the Path. There's no Close
// It, since closing a buffer could lose what's unsaved, which Neovim
// should ask about, not the launcher. open-now asks each Neovim over its
// own server, where it told its kitty window it listens (showing.lua).
//
// And in its Recent, beside the files it opened from the list programs
// share, the sessions it kept on leaving a folder with more than one file
// open (sessions.lua): each the folder's row, saying when beside the
// folder holding it, ordered among the rest by when, Return opening
// Neovim on it in a new kitty window there, with Forget It inside, which
// removes it and keeps the launcher. Its drawer in Programs says how many
// files are open in how many windows, how many are modified, and how many
// sessions are kept.
//
// What's open and the sessions are read ahead, as the shell starts and
// again as the launcher closes, and what's open again as Neovim is
// stepped into for what changed since, as kitty's tabs are (Kitty.qml),
// so each row is there from the first frame Neovim's page draws, never
// joining its end late. What each says is read in Neovim.js.
pragma ComponentBehavior: Bound

import qs
import Quickshell.Io
import QtQuick
import "../Copied.js" as Copying
import "Ago.js" as Ago
import "Neovim.js" as Neovim
import "NeovimOpening.js" as Opening
import "Repositories.js" as Repos

Provider {
    id: provider

    // The desktop entry Neovim's program row is made from.
    readonly property string program: "nvim"

    // The existing file's path the clipboard holds, or null.
    readonly property var place: Copied.place
    // Whether the last read found a repository to open a file of.
    readonly property bool repositories: Repositories.listed.length > 0
    // So Programs › Neovim, held while either comes or goes, draws it.
    onPlaceChanged: changed()
    onRepositoriesChanged: changed()

    // Found from the top by their names, each only while it's listed.
    foundActs: (repositories ? [{ program: program, key: "verb:nvim:find" }] : [])
        .concat(place !== null ? [{ program: program, key: "verb:nvim:copied" }] : [])

    // What each Neovim holds, as open-now last said, and whether it was
    // asked since the launcher opened; the sessions kept, as find last
    // listed them, and those forgotten since, left out until the next
    // listing no longer has them.
    property var neovims: []
    property bool looked: false
    property var sessions: []
    property var forgotten: []
    // A forgetting asked for while one runs, run once it's done.
    property bool forgetAgain: false

    Component.onCompleted: {
        Repositories.want(provider.settings, false);
        Repositories.opened(provider.launcher?.open ?? false);
        provider.readAhead();
    }
    // The settings name the programs each read runs, and may be given after
    // the provider is made.
    onSettingsChanged: provider.readAhead()

    function start(readers) {
        for (const reader of readers) {
            if ((reader.command[0] ?? "") !== "" && !reader.running)
                reader.running = true;
        }
    }

    function readAhead() {
        provider.start([asking, listing]);
    }

    // Asked again as Neovim is stepped into, once an opening.
    function look() {
        if (looked)
            return;
        looked = true;
        provider.start([asking]);
    }

    // Each answer draws Neovim's step again, once a frame however many
    // come together.
    function heard() {
        Qt.callLater(provider.redraw);
    }

    function redraw() {
        provider.changed();
    }

    Process {
        id: asking

        command: [provider.settings.openNow ?? "", "neovim"]
        running: false
        stdout: StdioCollector {
            id: asked

            onStreamFinished: {
                provider.neovims = Neovim.heard(asked.text);
                provider.heard();
            }
        }
    }

    Process {
        id: listing

        command: [provider.settings.find ?? "", provider.settings.sessions ?? "", "-maxdepth",
            "1", "-type", "f", "-name", "*.vim", "-printf", "%T@\\t%f\\n"]
        running: false
        stdout: StdioCollector {
            id: listed

            onStreamFinished: {
                const read = Neovim.sessions(listed.text, provider.settings.sessions ?? "");
                provider.sessions = read;
                provider.forgotten = provider.forgotten.filter(path => read
                    .some(one => one.path === path));
                provider.heard();
            }
        }
    }

    Process {
        id: forgetting

        command: [provider.settings.rm ?? "", "-f", "--"].concat(provider.forgotten)
        running: false
        onExited: {
            if (provider.forgetAgain) {
                provider.forgetAgain = false;
                forgetting.running = true;
            } else {
                provider.start([listing]);
            }
        }
    }

    // What's read arrives while a repository's files may be shown.
    Connections {
        target: Repositories

        function onChanged() {
            provider.changed();
        }
    }

    // The launcher closed, so the next step into Neovim asks it again, and
    // what's open and the sessions are read for the next opening.
    Connections {
        target: provider.launcher

        function onOpenChanged() {
            Repositories.opened(provider.launcher.open);
            if (!provider.launcher.open) {
                provider.looked = false;
                provider.readAhead();
            }
        }
    }

    function programActs(asked) {
        if (asked !== program)
            return [];
        provider.look();
        return (repositories ? [finding()] : []).concat(place !== null ? [copied()] : [],
            provider.openNow().map(one => provider.openRow(one)));
    }

    // Its sessions, for its Recent (Applications.qml asks every provider's
    // recentOf), newest first, each dated so Recent orders them among the
    // files it opened.
    function recentOf(asked) {
        if (asked !== program)
            return [];
        return provider.sessionsNow().map(one => provider.sessionRow(one));
    }

    // What its drawer in Programs says it holds (Applications.qml asks
    // every provider's programFacts), from what was read ahead, never a
    // read of its own.
    function programFacts(asked) {
        if (asked !== program)
            return [];
        return Neovim.held(provider.neovims, provider.openNow(), provider.sessionsNow(),
            provider.said()).map(fact => ({ label: Words.name(fact.id), value: fact.value }));
    }

    // What its rows and drawer say of what's open.
    function said() {
        return Words.of("nvim:files").said ?? {};
    }

    // Every file open now, each once, the Neovim looked at last first.
    function openNow() {
        return Neovim.open(provider.neovims, window => provider.focusedAt(window));
    }

    // When niri's window was last looked at, or 0 for one gone or never.
    function focusedAt(id) {
        const stamp = Niri.windows[id]?.focus_timestamp;
        return stamp ? stamp.secs * 1e9 + stamp.nanos : 0;
    }

    // A file open now, as the file's one row (Files.held), its state
    // before its folder; Return goes to it.
    function openRow(one) {
        const key = "verb:path:file:go";
        const goes = thing({
            key: `${key}:${one.path}`,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            run: () => provider.go(one)
        });
        return Files.held(one.path, thing, [goes], Neovim.side(one, provider.said()));
    }

    // Goes to a file: its kitty window and the buffer inside it through
    // open-now, then that window through niri.
    function go(one) {
        Niri.spawn(Neovim.going(provider.settings.openNow ?? "", one));
        Niri.act({ FocusWindow: { id: one.neovim.window } });
    }

    function sessionsNow() {
        return provider.sessions.filter(one => !provider.forgotten.includes(one.path));
    }

    // One of a session's acts, as the vocabulary says it, keyed by what
    // it's done to, with its : command.
    function sessionAct(id, one, fields) {
        return thing(Object.assign({
            key: `${id}:${one.path}`,
            name: Words.name(id),
            verb: Words.verb(id),
            glyph: Words.glyph(id),
            hue: "action"
        }, Words.ending(id), fields));
    }

    // A session as the folder's row (Files.qml), saying when it was kept
    // before the folder holding it; Return opens it, and Forget It inside
    // takes it away.
    function sessionRow(one) {
        const opens = provider.sessionAct("verb:nvim:session:open", one, {
            run: () => Niri.spawn(Opening.inKitty(Opening.kitty(Object.values(Niri.windows)),
                one.folder, Opening.session(one.path)))
        });
        const forgets = provider.sessionAct("verb:nvim:session:forget", one, {
            stays: true,
            run: () => provider.forget(one)
        });
        const side = Neovim.sessionSide(one, Date.now(), Files.within(one.folder));
        const sided = fields => thing(fields.what === "folder"
            ? Object.assign(fields, { side: side }) : fields);
        return Object.assign(Files.folder(one.folder, sided, [opens, forgets]), { at: one.at });
    }

    // Removes a session's file, leaving its row out at once.
    function forget(one) {
        provider.forgotten = provider.forgotten.concat([one.path]);
        provider.changed();
        if (forgetting.running)
            provider.forgetAgain = true;
        else
            forgetting.running = true;
    }

    // Open a File by Name, saying how many repositories it holds, and,
    // stepped into once a read finds none left, that there are none.
    function finding() {
        const key = "verb:nvim:find";
        return thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            side: String(Repositories.listed.length),
            ask: Words.of(key).ask ?? "",
            empty: (Words.of(key).empty ?? "").replace("{folder}",
                Files.shown(Repositories.folder)),
            nested: true,
            targets: () => {
                const now = Date.now();
                return Repositories.listed.map(one => provider.repository(one, now));
            }
        });
    }

    // A repository as the folder it is, saying when its git last wrote,
    // which Return steps into, its files listed there.
    function repository(one, now) {
        const row = Files.folder(one.path, thing, [], () => provider.filesOf(one.path));
        return Object.assign(row, { side: Ago.ago(one.at, now), nested: true });
    }

    // A repository's files, each as the file it is beside its folder in the
    // repository, Return opening it there.
    function filesOf(folder) {
        Repositories.askFiles(folder);
        return (Repositories.files[folder] ?? []).map(file => Object.assign(
            Files.file(file.path, thing, [provider.opening(file.path, folder, 0, 0)]),
            { side: Repos.within(file.path, folder) }));
    }

    // Open the Copied Path, saying the path and its line as copied.
    function copied() {
        const key = "verb:nvim:copied";
        const where = place;
        return thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            side: Copying.placeSaid(where, Files.home),
            // What's copied when it runs, which a row found from the top,
            // made as the act came, may not have changed with.
            run: () => {
                const now = Copied.place;
                if (now !== null)
                    provider.open(now.path, now.folder, now.line, now.column);
            }
        });
    }

    // A file's Open, Recent's own word for opening it in the program.
    function opening(path, folder, line, column) {
        const key = "verb:path:file:open";
        return thing({
            key: `${key}:${path}`,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            run: () => provider.open(path, folder, line, column)
        });
    }

    function open(path, folder, line, column) {
        Niri.spawn(Opening.inKitty(Opening.kitty(Object.values(Niri.windows)), folder,
            Opening.neovim(path, line, column)));
    }
}
