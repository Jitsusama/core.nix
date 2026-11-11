pragma Singleton
// The git repositories under a folder, ~/src as the programs' modules give
// it, for the acts that list them: Neovim's Open a File by Name and
// Chrome's Open a Repository on GitHub. Both ask this one (`want`), so
// they're asked for once for the shell, not once each. The programs
// daemon reads them (modules/programs/src/repositories.rs) as the shell
// asks, as it starts and again each time the launcher opens (`again`),
// and never otherwise, so nothing is read at rest, and each act is listed
// only while the last read found one it can open; the one whose git last
// wrote first (Repositories.js). What's read last is kept while the next
// is read, so an act never goes and comes back as the launcher opens.
// Their remotes come with them, kept once an act wants them, so Chrome
// lists those on GitHub, and a repository's files as it's stepped into,
// the one changed last first, forgotten as the launcher closes. Each
// answer comes over the one socket the shell holds open to the daemon,
// starting no command here. Until Programs' own ready rows, the acts
// still build their rows from these lines here; the daemon takes that on
// with them. `changed` says when something read arrives.
import qs
import Quickshell
import QtQuick
import "Repositories.js" as Repos

Singleton {
    id: root

    // The folder they're under, as the programs' modules give it.
    property string folder: ""
    // Whether an act wants their remotes, as Chrome's does.
    property bool remoting: false
    // Whether the launcher is open, so a repository's files read as it
    // closed aren't kept for the next time it opens.
    property bool showing: false

    // Each repository, { path, at }, the one whose git last wrote first,
    // and whether they've been read at all.
    property var listed: []
    property bool read: false
    // Each repository's remotes, by its folder, as Repositories.js's
    // remotes gives them, and whether they've been read at all.
    property var remotes: ({})
    property bool remotesRead: false
    // Each repository's files, { path, at }, by its folder, the one
    // changed last first, as each was read since the launcher opened.
    property var files: ({})

    signal changed

    // An act's settings, { folder }, and whether it wants the remotes:
    // what's under the folder is read now, unless it is being read
    // already.
    function want(settings, remotesToo) {
        if ((settings.folder ?? "") !== "")
            root.folder = settings.folder;
        root.remoting = root.remoting || remotesToo === true;
        root.again();
    }

    // The launcher opening reads them again, and its closing forgets each
    // repository's files, which are read again as it's stepped into.
    function opened(open) {
        root.showing = open;
        if (open)
            root.again();
        else
            root.files = {};
    }

    // What the daemon last said of the folder, as again() compares it.
    property string said: ""

    // The folder being read, so two acts asking as the launcher opens ask
    // once, or "".
    property string reading: ""

    // Reads them now, unless they're being read.
    function again() {
        if (root.folder === "" || root.reading === root.folder)
            return;
        const key = `repositories:${root.folder}`;
        root.reading = root.folder;
        daemon.get([key], 0, () => {
            root.reading = "";
            const record = daemon.held[key];
            if (record === undefined)
                return;
            // Asked again at each opening, it says a change only where
            // there is one, since each act reading it is drawn again. A
            // stopgap: the programs daemon serves each repository ready and
            // tells only as one changes, and this compare goes, by Fri
            // 2026-11-13 12:00.
            const said = `${record.dated.join("\n")}\0${root.remoting ?
                record.configs.join("\n") : ""}`;
            if (root.read && said === root.said && (!root.remoting || root.remotesRead))
                return;
            root.said = said;
            root.listed = Repos.repositories(record.dated.join("\n"));
            root.read = true;
            if (root.remoting) {
                root.remotes = Repos.remotes(record.configs.join("\n"));
                root.remotesRead = true;
            }
            root.changed();
        });
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
            root.files = Object.assign({}, root.files,
                { [repository]: Repos.newest(Repos.dated(record.dated.join("\n"))) });
            root.changed();
        });
    }

    Daemon {
        id: daemon

        socket: `${Quickshell.env("XDG_RUNTIME_DIR") ?? ""}/daemons/programs.sock`
        kind: "repositories"
        live: false

        // Asked before the shell had reached it, as it starts, they're
        // asked again once it answers.
        onStateChanged: {
            if (daemon.state === "answering" && !root.read)
                root.again();
        }
    }
}
