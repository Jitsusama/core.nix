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
pragma ComponentBehavior: Bound

import qs
import QtQuick
import "../Copied.js" as Copying
import "Ago.js" as Ago
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

    Component.onCompleted: {
        Repositories.want(provider.settings, false);
        Repositories.opened(provider.launcher?.open ?? false);
    }

    // What's read arrives while a repository's files may be shown.
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
        return (repositories ? [finding()] : []).concat(place !== null ? [copied()] : []);
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
