// What a QML test weighing memory in Qt's own engine reads of its process:
// its own anonymous memory, from /proc. A test imports it as
// `import "Memory.js" as Memory`, its check copying it beside the test and
// setting QML_XHR_ALLOW_FILE_READ, since it reads /proc as a file.
.pragma library

// smaps_rollup's line for the process's own anonymous memory, in kilobytes.
const ANONYMOUS = /Pss_Anon:\s+(\d+)/;

// The process's own anonymous memory, in bytes.
function anonymous() {
    const request = new XMLHttpRequest();
    request.open("GET", "file:///proc/self/smaps_rollup", false);
    request.send();
    const kilobytes = ANONYMOUS.exec(request.responseText);
    if (!kilobytes) {
        throw new Error("/proc/self/smaps_rollup says no Pss_Anon");
    }
    return Number(kilobytes[1]) * 1024;
}
