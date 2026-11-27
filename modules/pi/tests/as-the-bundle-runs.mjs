// Not a test itself: given to node with --import before a test, it makes
// pi's unbundled code run as the bundle a window starts does. pi's bundle
// defines PI_BUNDLED_NODE, and its config reads it as a global, so with it
// set the extension loader takes jiti's static loader and serves pi's own
// packages from inside pi, as a windowed pi's loader does.
globalThis.PI_BUNDLED_NODE = true;
