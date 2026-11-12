// pi's sessions, for the launcher's Now (tests/now-moment.qml): where an
// item is done names its session, and For You lists the sessions wanting
// Joel, none of them parked, and those to reopen, each as the case gives;
// Go To says the session it went to, as it was handed it, and each steer
// is kept as it was sent, and said put or not as the case sets (puts).
pragma Singleton
import Quickshell
import QtQuick

Singleton {
    property var running: []
    property var wanting: []
    property var toReopen: []
    property var wentTo: null
    property var steered: []
    property bool puts: true

    function steer(address, fields, then) {
        steered = steered.concat([{ address: address, fields: fields }]);
        if (then)
            then(puts);
    }

    function wanted(wants) {
        return "needs you";
    }

    function goTo(session) {
        wentTo = session;
    }

    function parkedAs(address) {
        return null;
    }
}
