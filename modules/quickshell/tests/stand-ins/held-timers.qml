// The timers daemon's held timers, for the launcher's timers
// (tests/timers.qml): the timers and the stopwatch the case gives, each
// act asked kept as its name and the record's key, answered at once with
// the refusal the case sets, and a typed start with what the case started.
pragma Singleton
import Quickshell
import QtQuick

Singleton {
    property var timers: []
    property var stopwatch: null
    property var asked: []
    property var refusal: null
    property var started: null
    signal heard(var keys)
    function timer(key) { return null; }
    function asking(act, key, then) {
        asked = asked.concat([key === "" ? act : `${act} ${key}`]);
        then?.(refusal);
    }
    function start(span, name, then) { then?.(started); }
    function pause(key, then) { asking("pause", key, then); }
    function resume(key, then) { asking("resume", key, then); }
    function again(key, then) { asking("again", key, then); }
    function add(key, span, then) { asking("add", key, then); }
    function end(key, then) { asking("end", key, then); }
    function run(then) { asking("run", "", then); }
    function stop(then) { asking("stop", "", then); }
    function lap(then) { asking("lap", "", then); }
    function reset(then) { asking("reset", "", then); }
}
