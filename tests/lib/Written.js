// A file a QML test writes through a FileView of its own, written whole
// before the test goes on, so what the shell under test loads next is what
// the test meant. A test imports it as `import "Written.js" as Written`, its
// check copying it beside the test.
.pragma library

function written(file, text) {
    file.setText(text);
    file.waitForJob();
}
