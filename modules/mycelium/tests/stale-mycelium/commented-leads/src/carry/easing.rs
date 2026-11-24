// A stand-in for mycelium's source, which core's mycelium checks read
// (carry-guards.nix): naming `leads` only in comments.
// fn leads(here: &str) -> Option<String>
/// pub fn leads(here: &str) -> Option<String>
fn lead(here: &str) -> Option<String> {
    None
}
