# The Machine Program

`machine/` is the daemon itself, in a folder of its own so its parts sit
together beside the library rather than among its 40 files:
`main.rs` starts it and says how it reads and when, and each file beside
it holds one part, named for what that part does.
