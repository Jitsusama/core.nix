// The pixels the wallpaper's picture is read at on a screen, as
// tests/wallpaper.qml holds them to.
.pragma library

// Every PNG opens with these eight bytes, then its header's length and
// name, then its width and its height as four bytes each.
const PNG_SIGNATURE = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
const PNG_WIDTH_AT = 16;
const PNG_HEIGHT_AT = 20;
const PNG_SIZE_ENDS = 24;

// The scale niri gives the output named `name` in its answer to Outputs,
// or null where it names none.
function scaleOf(outputs, name) {
    const scale = outputs?.[name]?.logical?.scale;
    return typeof scale === "number" && scale > 0 ? scale : null;
}

// The size a PNG says it is in its header, from the file's bytes, or
// null where they aren't a PNG's.
function ownSize(bytes) {
    if (!(bytes instanceof ArrayBuffer) || bytes.byteLength < PNG_SIZE_ENDS) {
        return null;
    }
    const view = new DataView(bytes);
    if (PNG_SIGNATURE.some((byte, at) => view.getUint8(at) !== byte)) {
        return null;
    }
    return { width: view.getUint32(PNG_WIDTH_AT), height: view.getUint32(PNG_HEIGHT_AT) };
}

// The pixels a picture covering `screen` is read at: its logical size by
// niri's scale for its output where it's known, else by the screen's own
// ratio, which Wayland rounds up to a whole number, so 2 for 1.5.
// Filling by crop, Qt reads a picture as large as covers that both ways,
// even past the file's own size, so a picture whose `size` covers no more
// is read at that, which the GPU draws as well. A screen gone has no
// pixels.
function pixels(screen, scale, size) {
    const given = screen?.devicePixelRatio ?? 0;
    const ratio = scale ?? (given > 0 ? given : 1);
    const width = Math.round((screen?.width ?? 0) * ratio);
    const height = Math.round((screen?.height ?? 0) * ratio);
    if (size && Math.max(width / size.width, height / size.height) >= 1) {
        return { width: size.width, height: size.height };
    }
    return { width, height };
}
