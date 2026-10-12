pragma Singleton

// What the shell did, said for the desktop test while
// jitsusama.quickshell.reports is on: a line to the journal for each, as
// report[what], which the test waits on rather than guessing how long
// anything takes (tests/desktop.nix, told). Off, nothing is said.
//
// What's said is plain words, with no brackets or quotes inside, so the
// test can find each by its text alone. The launcher's state is the one
// exception: JSON after "showing ", read by its start.
import Quickshell
import QtQuick

Singleton {
    function say(what) {
        if (Options.reports)
            console.info(`report[${what}]`);
    }
}
