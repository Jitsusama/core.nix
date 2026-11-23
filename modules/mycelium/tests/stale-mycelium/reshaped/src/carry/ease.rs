// A stand-in for mycelium's source, which core's mycelium checks read
// (carry-guards.nix): after #381, its lines in forms the patterns accept.
pub struct Cell {
    pub(crate) driven: Option<bool>,
}
