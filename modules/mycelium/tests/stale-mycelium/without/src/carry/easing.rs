// A stand-in for mycelium's source, which core's mycelium checks read
// (carry-guards.nix): before #381, with no `leads`.
fn lead(here: &str) -> Option<String> {
    None
}
