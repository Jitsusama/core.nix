// pi's sessions, for the launcher's Now (tests/now-moment.qml): where an
// item is done names its session, and For You lists the sessions wanting
// Joel, none of them parked, and those to reopen, each as the case gives;
// Go To says the session it went to, as it was handed it, and each steer
// is kept as it was sent, and said put or not as the case sets (puts);
// and each Put It Off asked is kept as its session's address and the
// ask it names, which the case sets by address (knowns).
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
    property var putOffAsked: []
    property var knowns: ({})

    function putOffWant(address, known) {
        putOffAsked = putOffAsked.concat([`${address} ${known}`]);
    }

    function wantKnown(address) {
        return String(knowns[address] ?? "");
    }

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

    // Nothing here is parked, so going is all there is to it.
    function wakeToGo(resting, session) {
        goTo(session);
    }
}
