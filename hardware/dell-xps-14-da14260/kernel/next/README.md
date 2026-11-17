# The XPS 14's Kernel Patches No Release Has Yet

Fixes for the XPS 14's parts that the laptop's kernel release lacks, taken onto
it before the release that holds them. Each patch's first line ends with where
it stands, in brackets: mainline, linux-next, posted to a list, or local for
the one written here, which linux-next lacks too. The message under it names
the commits or the posting it carries, if any.

[`../next.nix`][next] applies them in its order, after `../local/`'s, and its
assertion fails once the kernel reaches the release that may hold them, which
is the sign to drop each patch that release has. [The kernel's
README][kernel] says how a patch moves between these folders.

[next]: ../next.nix
[kernel]: ../README.md
