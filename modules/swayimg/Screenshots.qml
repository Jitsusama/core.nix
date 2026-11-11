// swayimg's own in the launcher's Programs › swayimg: something to look
// through first, always, as its page leads with it (`leads`), so Return on
// swayimg lands on a gallery. While the screenshots folder holds a picture,
// that's its gallery, newest first and the newest selected, found by typing
// too, for a screenshot taken a while ago; Capture's Recent holds the
// newest ones taken. After it, or leading in its place while there are
// none, comes a gallery of each folder the pictures in swayimg's Recent
// are in, newest first, each once, its name saying the folder and its side
// how many pictures it holds and where, opening on the newest of Recent's
// pictures there (Swayimg.js), each found from the top by its name as well
// (`foundActs`). A row a folder rather than an act on each of Recent's
// pictures: several pictures from one folder would each hold the same
// gallery, and Recent's rows are the one row every surface shows of a
// file, which holds what's done to a file, never what's done to its
// folder. Nothing leads only when there's neither a screenshot nor a
// picture in Recent, since a gallery of nothing opens nothing.
//
// Recent is read where Programs reads it (Applications.qml's Recents), so
// these are the folders of the very pictures its Recent lists, kept up as
// that list is. How many pictures each folder holds is read by the
// programs daemon as swayimg is stepped into and on each change while it's
// held, never on a timer, so the shell starts no process for it. Its
// drawer says when the newest screenshot was taken and how many there are
// (`programFacts`), and each gallery's drawer is its folder's plate, as a
// folder's is wherever a file's is: where it is, then its kind, the newest
// picture previewed, its name, when it was taken or added, and how many
// there are (Swayimg.js's `plate`).
import qs
import Quickshell
import QtQuick
import "../FileFacts.js" as FileFacts
import "Swayimg.js" as Swayimg

Provider {
    id: provider

    // Swayimg's desktop entry hides itself, as it opens nothing without a
    // picture, so Programs lists it for its galleries and its Recent,
    // stepping in on Return rather than starting.
    hiddenPrograms: [program]

    // Its desktop entry, whose program these are given to.
    readonly property string program: "swayimg"

    // How many pictures the folder holds, and the newest, as the one reader
    // of what was taken says (Taken.qml), which is read as the launcher
    // opens.
    readonly property int count: Taken.countIn(settings.folder, /\.png$/i)
    readonly property var newest: Taken.newestIn(settings.folder, /\.png$/i)

    Component.onCompleted: Taken.want(settings.folder)

    // The pictures in swayimg's Recent, newest first, as Programs reads them
    // for it, or none where Programs isn't there to read them. The one it
    // shows now is among them, though Recent lists it as open instead,
    // since its folder is as much one to look through.
    readonly property var programs: (launcher?.sources ?? [])
        .find(other => other !== provider && typeof other.recents?.of === "function") ?? null
    readonly property var recent: programs?.recents.of(program) ?? []

    // The screenshots' folder isn't offered twice while its own gallery is.
    readonly property var skipped: count > 0 ? [settings.folder] : []

    // How many pictures each of Recent's folders held when last read, or
    // null before the first read.
    property var listed: null
    readonly property var galleries: Swayimg.galleries(recent, listed, skipped)
    // So Programs › swayimg, held as a count comes or a folder goes, is
    // drawn again.
    onGalleriesChanged: changed()

    // The folders of Recent's pictures, each gallery found from the top by
    // its name too, made again only as a folder comes or goes.
    readonly property var recentFolders: Swayimg.folders(recent, skipped).map(one => one.folder)
    foundActs: recentFolders.map(folder => ({
        program: provider.program,
        key: provider.keyOf(folder)
    }))

    // Whether swayimg's own step is held, from Programs or from the top.
    readonly property bool steppedIn: (launcher?.open ?? false)
        && (launcher?.held ?? []).some(step => step.row?.key === `program:${provider.program}`)
    // Whether Recent's folders are watched: from once the step is drawn
    // (Launcher.qml's afterFrame), so its frame never waits on an answer,
    // until it's left or the launcher closes.
    property bool watchingFolders: false

    onSteppedInChanged: {
        if (steppedIn)
            launcher.afterFrame(() => provider.watchingFolders = provider.steppedIn);
        else
            watchingFolders = false;
    }

    // What's in Recent's folders, as the programs daemon reads them itself,
    // as each watch of them begins and on each change while it's held, and
    // never otherwise; a folder Recent gains while it's held is watched as
    // it comes.
    Daemon {
        id: folders

        socket: `${Quickshell.env("XDG_RUNTIME_DIR") ?? ""}/daemons/programs.sock`
        kind: "folder"
        live: false
        ahead: provider.watchingFolders ? provider.recentFolders.map(Swayimg.folderKey) : []

        onHeard: provider.hear()
    }

    // What the folders said when last counted, so a watch begun again,
    // whose whole answer says every folder anew, remakes no gallery when
    // nothing in them changed.
    property string heardAs: ""

    // Counts the folders the daemon has said, leaving one it hasn't yet
    // uncounted.
    function hear() {
        const within = recentFolders.filter(folder => Swayimg.folderKey(folder) in folders.held);
        const lines = within.reduce((all, folder) => all.concat(
            folders.held[Swayimg.folderKey(folder)].lines ?? []), []);
        const said = JSON.stringify([within, lines]);
        if (said === heardAs)
            return;
        heardAs = said;
        listed = Swayimg.read(lines, within);
    }

    // How a gallery's count is said, and when the newest screenshot was
    // taken, in the launcher's words and the locale's days, as everywhere
    // (Y2YBTS, 2026-11-03).
    readonly property var pictures: Words.of("swayimg:screenshots").said ?? {}
    readonly property var factWords: FileFacts.wordsOf(id => Words.of(id),
        (at, format) => Qt.formatDate(at, format))
    // And what a gallery's drawer says, each fact's label by its id.
    readonly property var plateWords: ({
            file: factWords,
            labelled: id => Words.name(id)
        })

    // A gallery's drawer, drawn as it's shown, so when the newest came is
    // said from the moment it's looked at.
    function plate(folder, gallery) {
        return Swayimg.plate(Object.assign({
            where: Files.within(folder),
            name: Files.named(folder)
        }, gallery), Date.now(), provider.plateWords);
    }

    readonly property var looking: thing({
        key: "swayimg:screenshots",
        verb: Words.verb("swayimg:screenshots"),
        name: Words.name("swayimg:screenshots"),
        keywords: "pictures images gallery screenshot",
        glyph: programGlyph(program),
        hue: "action",
        side: Swayimg.side(provider.count, "", provider.pictures),
        leads: true,
        later: true,
        // A thing with a drawer, the folder it looks through.
        thing: true,
        drawer: () => provider.plate(provider.settings.folder, {
            count: provider.count,
            newest: provider.newest,
            taken: true
        }),
        // Newest first, as the folder's names may not sort that way: niri's
        // and the shell's region pictures are named differently.
        run: () => Niri.spawn(Swayimg.opening(provider.settings.gallery,
            provider.settings.folder))
    })

    things: provider.count > 0 ? [looking] : []

    // The key of a folder's gallery, by its whole path.
    function keyOf(folder) {
        return `verb:swayimg:folder:${folder}`;
    }

    // One folder's gallery, leading the page where `leads` says.
    function galleryRow(one, leads) {
        const words = "verb:swayimg:folder";
        return thing({
            key: keyOf(one.folder),
            verb: Words.verb(words),
            name: Swayimg.named(Words.name(words), Files.named(one.folder)),
            keywords: "pictures images gallery folder",
            glyph: programGlyph(program),
            hue: "action",
            side: Swayimg.side(one.count, Files.within(one.folder), provider.pictures),
            leads: leads,
            later: true,
            thing: true,
            drawer: () => provider.plate(one.folder, {
                count: one.count,
                newest: one.newest,
                target: one.target,
                taken: false
            }),
            run: () => Niri.spawn(Swayimg.opening(provider.settings.gallery, one.target))
        });
    }

    // The picture viewer's own acts in Programs: its galleries, the one
    // that leads first.
    function programActs(asked) {
        if (asked !== program)
            return [];
        const lead = Swayimg.lead(count, galleries);
        return provider.things.concat(galleries.map(one => galleryRow(one, one.folder === lead)));
    }

    // What its drawer in Programs says of the screenshots (Applications.qml
    // asks every provider's programFacts), from the one reader of what was
    // taken.
    function programFacts(asked) {
        if (asked !== program)
            return [];
        return Swayimg.held(newest, count, Date.now(), factWords)
            .map(fact => ({ label: Words.name(fact.id), value: fact.value }));
    }
}
