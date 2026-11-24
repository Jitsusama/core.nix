// A stand-in for mycelium's source, which core's mycelium checks read
// (carry-guards.nix): after #381, its lines in forms the patterns accept.
pub(crate) fn leads<'a>(here: &'a str) -> Option<&'a str> {
    None
}
