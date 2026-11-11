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
// Each file's drawer says where it is then its kind, previews its first
// lines and counts them, as the file chooser says a file (FileFacts.js)
// once file-preview has read it as saved, and then what only Neovim
// knows: its buffer's number, as :b takes it, and when it was last in a
// window (Neovim.js's plateFacts).
//
// Its drawer in Programs says how many files are open in how many
// windows, and how many are modified. It only reads what each Neovim
// says over its own server and changes nothing of Neovim's, so the
// editor stays the account's own.
//
// What's open is read by the programs daemon, which runs open-now as its
// `listed:nvim:open` list (home.nix), so the page starts no process to
// read it (Joel, 2026-11-10: the daemons are the data layer): once as the
// shell starts and again each time the launcher opens, so each row is there
// from the first frame Neovim's page draws, never joining its end late,
// what was read last showing until the daemon answers. What each says is
// read in Neovim.js. The page still makes its rows from those lines, a
// stopgap until Programs serves its own ready rows (Thu 11-12).
pragma ComponentBehavior: Bound

import qs
import Quickshell.Io
import QtQuick
import "../Copied.js" as Copying
import "../FileFacts.js" as FileFacts
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

    // What each Neovim holds, as open-now last said.
    property var neovims: []
    // What open-now last said, word for word, so an answer the same as the
    // last draws nothing again.
    property string lastHeard: ""

    // What a file's drawer says of the file, as the file chooser says it,
    // in the launcher's words.
    readonly property var fileWords: FileFacts.wordsOf(id => Words.of(id),
        (at, format) => Qt.formatDate(at, format))
    // What file-preview said of each file whose drawer was drawn, from the
    // one kept, forgotten as the launcher closes, since a file open in
    // Neovim changes.
    readonly property var previews: previewing
    Previewer {
        id: previewing

        command: provider.settings.preview ?? []
    }

    Component.onCompleted: {
        Repositories.want(provider.settings, false);
        Repositories.opened(provider.launcher?.open ?? false);
    }

    // Each answer draws Neovim's step again, once a frame however many
    // come together.
    function heard() {
        Qt.callLater(provider.redraw);
    }

    function redraw() {
        provider.changed();
    }

    // What open-now printed, as the programs daemon holds it under the key
    // the settings name, read as the shell starts and each time the
    // launcher opens.
    Listed {
        id: listing

        key: provider.settings.open ?? ""
        wanted: (provider.launcher?.open ?? false) || !listing.read

        onHeard: {
            const text = listing.lines.join("\n");
            if (text === provider.lastHeard)
                return;
            provider.lastHeard = text;
            provider.neovims = Neovim.heard(text);
            provider.heard();
        }
    }

    // What's read arrives while a repository's files may be shown.
    Connections {
        target: Repositories

        function onChanged() {
            provider.changed();
        }
    }

    // The repositories follow the launcher opening and closing, and the
    // previews are let go as it closes; what's open is the programs
    // daemon's, read through the listing above.
    Connections {
        target: provider.launcher

        function onOpenChanged() {
            Repositories.opened(provider.launcher.open);
            if (!provider.launcher.open)
                previewing.forget();
        }
    }

    function programActs(asked) {
        if (asked !== program)
            return [];
        return foundActsOf(asked).concat(provider.openNow().map(one => provider.openRow(one)));
    }

    // Only the acts found from the top, which Programs asks inside a
    // binding: asking open-now there wrote what the binding read, a loop on
    // Joel's shell at each opening and closing, and the open files' rows
    // read every window's focus, so each focus made every act found from
    // the top again (2026-11-09).
    function foundActsOf(asked) {
        if (asked !== program)
            return [];
        return (repositories ? [finding()] : []).concat(place !== null ? [copied()] : []);
    }

    // What it lists as open now, by the keys Programs gives a file's row
    // (Applications.qml's keyOf), so Neovim's Recent leaves each out and
    // its note counts them, from what was read ahead.
    function programOpen(asked) {
        if (asked !== program)
            return [];
        return provider.openNow().map(one => `file:${one.path}`);
    }

    // What its drawer in Programs says it holds (Applications.qml asks
    // every provider's programFacts), from what was read ahead, never a
    // read of its own.
    function programFacts(asked) {
        if (asked !== program)
            return [];
        return Neovim.held(provider.neovims, provider.openNow(), provider.said())
            .map(fact => ({ label: Words.name(fact.id), value: fact.value }));
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
        const row = Files.held(one.path, thing, [goes], Neovim.side(one, provider.said()));
        row.drawer = () => provider.plate(one, row.name);
        return row;
    }

    // A file's drawer: where it is and its kind, its name, its first lines
    // and how many once file-preview has said, asked a frame after it's
    // first drawn, never while the drawer's binding reads, since that
    // would change what it reads; then what only Neovim knows of it.
    function plate(one, name) {
        const shown = previewing.shown(one.path);
        const file = { path: one.path, name: name, where: Files.within(one.path), entry: null };
        const drawn = FileFacts.plate(file, shown ?? null, provider.fileWords);
        const now = Date.now();
        const own = Neovim.plateFacts(one, at => FileFacts.when(at, now, provider.fileWords));
        drawn.facts = drawn.facts.concat(own.map(fact => ({
            label: Words.name(fact.id),
            value: fact.value
        })));
        return drawn;
    }

    // Goes to a file: its kitty window and the buffer inside it through
    // open-now, then that window through niri.
    function go(one) {
        Niri.spawn(Neovim.going(provider.settings.openNow ?? "", one));
        Niri.act({ FocusWindow: { id: one.neovim.window } });
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
