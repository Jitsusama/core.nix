# Screen Picture

The wallpaper's picture as its screen goes and comes back, in an offscreen
Quickshell, for the `screen-picture-follows-its-screen` check. It copies the
shell's `ScreenPicture.qml` and `Wallpaper.js` beside these, with a picture
of 1 by 1, and fails on any warning.

| File        | Stands in for, or does                                              |
| ----------- | ------------------------------------------------------------------- |
| `shell.qml` | shows the picture, takes its screen away and gives it back, counting reads |
| `Niri.qml`  | a niri that refuses every request, so the screen's own ratio stands |
