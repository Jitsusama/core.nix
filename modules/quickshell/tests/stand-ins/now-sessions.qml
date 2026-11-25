// pi's sessions, for the launcher's Now (tests/now-moment.qml): where an
// item is done names its session, and For You lists the sessions wanting
// Joel, those parked, and those to reopen, each as the case gives;
// Go To says the session it went to, as it was handed it, and each steer
// is kept as it was sent, and said put or not as the case sets (puts),
// a node's answer given with it, a refusal where not put, and sent or
// not as the case sets (reaches), as Sessions.qml's steer says them,
// or, while the case holds them (holds), kept unanswered (held) for it
// to answer later, as a node slow to say comes back after other keys;
// and each Put It Off asked is kept as its session's address and the
// ask it names, which the case sets by address (knowns).
pragma Singleton
import Quickshell
import QtQuick

Singleton {
    property var running: []
    property var parked: []
    property var wanting: []
    property var toReopen: []
    property var wentTo: null
    // Whether Go To goes, false as the real one says where it told why it couldn't.
    property bool goes: true
    property var woke: null
    property var steered: []
    property bool puts: true
    property bool holds: false
    property bool reaches: true
    property var held: []
    property var putOffAsked: []
    // The addresses whose wants are put off, in capitals, as Sessions.qml's
    // putOff says them; the case sets it.
    property var putOff: ({})
    property var knowns: ({})

    function putOffWant(address, known) {
        putOffAsked = putOffAsked.concat([`${address} ${known}`]);
    }

    function wantKnown(address) {
        return String(knowns[address] ?? "");
    }

    function steer(address, fields, then) {
        steered = steered.concat([{ address: address, fields: fields }]);
        if (then && holds)
            held = held.concat([then]);
        else if (then)
            then(puts && reaches, !reaches ? null
                : puts ? { steered: { machine: "here" } } : { error: { message: "no" } }, reaches);
    }

    function wanted(wants) {
        return "needs you";
    }

    function goTo(session) {
        wentTo = session;
        return goes;
    }

    function parkedAs(address) {
        const upper = String(address).toUpperCase();
        return parked.find(one => (one.presence?.addresses ?? [one.address])
            .some(each => String(each).toUpperCase() === upper)) ?? null;
    }

    // Keeps whom it woke, as the real one wakes only one parked (resting).
    function wakeToGo(resting, session) {
        woke = resting;
        return goTo(session);
    }
}
