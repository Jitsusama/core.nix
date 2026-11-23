// A stand-in for mycelium's source, which core's mycelium checks read
// (carry-guards.nix): with `leads`, so only ease.rs's commented `driven`
// can fail it.
fn leads(here: &str) -> Option<String> {
    None
}
