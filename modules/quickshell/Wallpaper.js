// The pixels the wallpaper's picture is read at on a screen, as
// tests/wallpaper.qml holds them to.
.pragma library

// The scale niri gives the output named `name` in its answer to Outputs,
// or null where it names none.
function scaleOf(outputs, name) {
    const scale = outputs?.[name]?.logical?.scale;
    return typeof scale === "number" && scale > 0 ? scale : null;
}

// The pixels a picture covering `screen` is read at: its logical size by
// niri's scale for its output where it's known, else by the screen's own
// ratio, which Wayland rounds up to a whole number, so 2 for 1.5.
// A screen gone has no pixels.
function pixels(screen, scale) {
    const own = screen?.devicePixelRatio ?? 0;
    const ratio = scale ?? (own > 0 ? own : 1);
    return { width: Math.round((screen?.width ?? 0) * ratio),
        height: Math.round((screen?.height ?? 0) * ratio) };
}
