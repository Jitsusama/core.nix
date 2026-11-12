pragma Singleton
// The git repositories under a folder, ~/src as the programs' modules give
// it, for the acts that list them: Neovim's Open a File by Name, Chrome's
// Open a Repository on GitHub and kitty's page. Each asks this one
// (`want`), so they're watched once for the shell, not once each. The
// programs daemon serves them ready (modules/programs/src/repositories.rs):
// each repository's path, when its git last wrote and its page on GitHub,
// the one written last first, and those on GitHub again on their own. It
// reads them as the watch begins and again only as one is cloned or its
// git writes, and tells the shell only then, so nothing is read at rest or
// as the launcher opens, and each act is listed only while they hold one
// it can open. A repository's files come as it's stepped into, each with
// the folder it's in there, the one changed last first, forgotten as the
// launcher closes. Each answer comes over the one socket the shell holds
// open to the daemon, starting no command here. `changed` says when
// something read arrives.
import qs
import Quickshell
import QtQuick

Singleton {
    id: root

    // The folder they're under, as the programs' modules give it.
    property string folder: ""
    // Whether the launcher is open, so a repository's files read as it
    // closed aren't kept for the next time it opens.
    property bool showing: false
    // The key their record is held by, or "" before any act wants them.
    readonly property string key: folder === "" ? "" : `repositories:${folder}`

    // Each repository, { path, at, page }, as the daemon serves them, those
    // on GitHub alike in `pages`, and whether they've been read at all.
    property var listed: []
    property var pages: []
    property bool read: false
    // Each repository's files, { path, at, within }, by its folder, as each
    // was read since the launcher opened.
    property var files: ({})

    signal changed

    // An act's settings, { folder }: what's under the folder is watched
    // from now on.
    function want(settings) {
        if ((settings.folder ?? "") !== "")
            root.folder = settings.folder;
    }

    // The launcher closing forgets each repository's files, which are read
    // again as it's stepped into.
    function opened(open) {
        root.showing = open;
        if (!open)
            root.files = {};
    }

    // What the daemon says of the folder, taken as it's said.
    function took() {
        const record = daemon.held[root.key];
        if (record === undefined)
            return;
        root.listed = record.repositories ?? [];
        root.pages = record.pages ?? [];
        root.read = true;
        root.changed();
    }

    // The repositories whose files are being read, so each is asked for
    // once.
    property var filing: ({})

    // One repository's files, unless they've been read since the launcher
    // opened or are being read.
    function askFiles(repository) {
        if (files[repository] !== undefined || filing[repository] === true)
            return;
        const key = `files:${repository}`;
        filing[repository] = true;
        daemon.get([key], 0, () => {
            delete filing[repository];
            const record = daemon.held[key];
            if (record === undefined || !root.showing)
                return;
            root.files = Object.assign({}, root.files, { [repository]: record.files ?? [] });
            root.changed();
        });
    }

    Daemon {
        id: daemon

        socket: `${Quickshell.env("XDG_RUNTIME_DIR") ?? ""}/daemons/programs.sock`
        kind: "repositories"
        live: false
        // Watched ahead for as long as the shell runs, so the daemon tells
        // of them only as they change; `heard` says a key only where its
        // record differs from what was held.
        ahead: root.key === "" ? [] : [root.key]

        onHeard: said => {
            if (said.includes(root.key))
                root.took();
        }
    }
}
