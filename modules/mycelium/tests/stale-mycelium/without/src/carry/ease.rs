// A stand-in for mycelium's source, which core's mycelium checks read
// (carry-guards.nix): before #381, with no `driven`.
pub struct Cell {
    pub fence: Option<Kind>,
}
