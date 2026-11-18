// The shell's notices alone, from the account's own files beside it, for a
// check that runs them on a real niri (tests/notices-on-niri.sh) rather than
// in a machine. The launcher isn't here, so a notice's history has nowhere
// to open.
import Quickshell
import QtQuick

ShellRoot {
    Notifications {}
}
