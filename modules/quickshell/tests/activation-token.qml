// A shell that only asks niri for an activation token, as the shell will as
// a notice's action is chosen once another process serves notices, and says
// what it was given, for a check on a real niri
// (tests/activation-token-on-niri.sh).
import Quickshell
import QtQuick

ShellRoot {
    Component.onCompleted: Quickshell.activationToken(token => console.log(`token[${token}]`))
}
