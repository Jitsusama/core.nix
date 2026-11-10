// What a QML test's stand-in daemon tells a watch the shell holds on it:
// a change, as one line of the daemon interface under the watch's id. A
// test imports it as `import "Watch.js" as Watch`, its check copying it
// beside the test, and passes the watch as it was asked, { id, socket }.
.pragma library

function tell(watch, change) {
    watch.socket.write(`${JSON.stringify(Object.assign({ id: watch.id }, change))}\n`);
    watch.socket.flush();
}
