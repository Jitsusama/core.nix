// The git repositories under a folder, ~/src as the programs' modules give
// it, for the acts that list them: Neovim's Open a File by Name and
// Chrome's Open a Repository on GitHub. They're read as such an act is
// stepped into, never before, and once each time the launcher opens, the
// one whose git last wrote first (Repositories.js); a repository's files
// as it's stepped into, the one changed last first; and only where asked,
// each one's remotes, so Chrome lists those on GitHub. fd finds them,
// each repository's .git three folders deep at most, and stat dates
// them, in one command each time, so a hundred repositories start no
// hundred commands. `changed` says when something asked for arrives.
import Quickshell
import Quickshell.Io
import QtQuick
import "Repositories.js" as Repos

Scope {
    id: root

    // The folder they're under, and the commands that find and date them,
    // by their paths, as the programs' modules give them.
    property string folder: ""
    property string fd: "fd"
    property string stat: "stat"
    property string grep: "grep"
    // Whether the launcher is open, so what's read is read again the next
    // time it opens.
    property bool showing: false

    // Each repository, { path, at }, the one whose git last wrote first,
    // and whether they've been read since the launcher opened.
    property var listed: []
    property bool read: false
    // Each repository's remotes, by its folder, as Repositories.js's
    // remotes gives them, and whether they've been read.
    property var remotes: ({})
    property bool remotesRead: false
    // Each repository's files, { path, at }, by its folder, the one
    // changed last first, as each was read since the launcher opened.
    property var files: ({})

    signal changed

    onShowingChanged: {
        if (showing)
            return;
        read = false;
        remotesRead = false;
        files = {};
    }

    // The repositories, read now unless they have been since it opened.
    function ask() {
        if (!read && !listing.running)
            listing.running = true;
    }

    // Their remotes, the same.
    function askRemotes() {
        if (!remotesRead && !configs.running)
            configs.running = true;
    }

    // One repository's files, the same; asked for another while one is
    // read, it's read next.
    function askFiles(repository) {
        const reading = filing.running && filing.repository === repository;
        if (files[repository] !== undefined || reading)
            return;
        next = repository;
        again();
    }

    // The repository whose files are asked for next, or "".
    property string next: ""

    // Starts reading the next repository's files once the last read is
    // done with, both its command ended and its files kept under its name,
    // whichever of the two comes second, so they're never kept under the
    // next one's.
    function again() {
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
                // Read as the launcher closed, they're read again as it
                // opens, since they may have changed between.
                root.read = root.showing;
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
                root.remotesRead = root.showing;
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
        onExited: root.again()
        stdout: StdioCollector {
            id: filed

            onStreamFinished: {
                if (root.showing)
                    root.files = Object.assign({}, root.files,
                        { [filing.repository]: Repos.dated(filed.text) });
                filing.kept = true;
                root.changed();
                root.again();
            }
        }
    }
}
