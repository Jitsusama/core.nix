pragma Singleton
// The git repositories under a folder, ~/src as the programs' modules give
// it, for the acts that list them: Neovim's Open a File by Name and
// Chrome's Open a Repository on GitHub. Both ask this one (`want`), so
// they're read once for the shell, not once each. They're read as the
// shell starts and again each time the launcher opens (`read`), as what
// was taken is (Taken.qml), and never otherwise, so no folder is watched
// at rest, and each act is listed only while the last read found one it
// can open; the one whose git last wrote first (Repositories.js). What's
// read last is kept while the next is read, so an act never goes and
// comes back as the launcher opens. Each one's remotes are read the same
// way, only once an act wants them, so Chrome lists those on GitHub, and
// a repository's files as it's stepped into, the one changed last first,
// forgotten as the launcher closes. fd finds them, each repository's .git
// three folders deep at most, stat dates them and grep reads their
// remotes, in one command each, so a hundred repositories start no
// hundred commands. `changed` says when something read arrives.
import Quickshell
import Quickshell.Io
import QtQuick
import "Repositories.js" as Repos

Singleton {
    id: root

    // The folder they're under, and the commands that find, date and read
    // them, by their paths, as the programs' modules give them.
    property string folder: ""
    property string fd: "fd"
    property string stat: "stat"
    property string grep: "grep"
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

    // An act's settings, { folder, fd, stat, grep }, each kept where it's
    // given, since Neovim's gives no grep, and whether it wants the
    // remotes: what's under the folder is read now, unless it is being
    // read already.
    function want(settings, remotesToo) {
        for (const name of ["folder", "fd", "stat", "grep"]) {
            if ((settings[name] ?? "") !== "")
                root[name] = settings[name];
        }
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

    // Reads them now, unless they're being read: two acts asking as the
    // launcher opens start one read.
    function again() {
        if (root.folder === "")
            return;
        if (!listing.running)
            listing.running = true;
        if (root.remoting && !configs.running)
            configs.running = true;
    }

    // One repository's files, unless they've been read since the launcher
    // opened; asked for another while one is read, it's read next.
    function askFiles(repository) {
        const reading = filing.running && filing.repository === repository;
        if (files[repository] !== undefined || reading)
            return;
        next = repository;
        nextFiles();
    }

    // The repository whose files are asked for next, or "".
    property string next: ""

    // Starts reading the next repository's files once the last read is
    // done with, both its command ended and its files kept under its name,
    // whichever of the two comes second, so they're never kept under the
    // next one's.
    function nextFiles() {
        if (filing.running || !filing.kept || next === "")
            return;
        filing.repository = next;
        next = "";
        filing.kept = false;
        filing.running = true;
    }

    // Every .git, a folder or a worktree's file, dated as stat gives it.
    Process {
        id: listing

        command: [root.fd, "--hidden", "--no-ignore", "--max-depth", "3", "--glob", ".git",
            root.folder, "--exec-batch", root.stat, "--printf", "%Y\\t%n\\n"]
        stdout: StdioCollector {
            id: listed

            onStreamFinished: {
                root.listed = Repos.repositories(listed.text);
                root.read = true;
                root.changed();
            }
        }
    }

    // Every repository's config, each section's heading and each url as
    // grep finds them, its path before each line.
    Process {
        id: configs

        command: [root.fd, "--hidden", "--no-ignore", "--max-depth", "4", "--full-path", "--glob",
            "**/.git/config", root.folder, "--exec-batch", root.grep, "-H", "-E",
            "^[[:space:]]*(\\[|url[[:space:]]*=)"]
        stdout: StdioCollector {
            id: configured

            onStreamFinished: {
                root.remotes = Repos.remotes(configured.text);
                root.remotesRead = true;
                root.changed();
            }
        }
    }

    // A repository's files as git would list them, its hidden ones too
    // but not what it ignores, nor git's own, dated as stat gives them.
    Process {
        id: filing

        property string repository: ""
        // Whether what it read last is kept, so the next can start.
        property bool kept: true

        command: [root.fd, "--type", "f", "--hidden", "--exclude", ".git", ".",
            filing.repository, "--exec-batch", root.stat, "--printf", "%Y\\t%n\\n"]
        onExited: root.nextFiles()
        stdout: StdioCollector {
            id: filed

            onStreamFinished: {
                if (root.showing)
                    root.files = Object.assign({}, root.files,
                        { [filing.repository]: Repos.newest(Repos.dated(filed.text)) });
                filing.kept = true;
                root.changed();
                root.nextFiles();
            }
        }
    }
}
