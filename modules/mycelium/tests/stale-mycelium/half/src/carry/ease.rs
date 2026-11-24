// A stand-in for mycelium's source, which core's mycelium checks read
// (carry-guards.nix): with `driven` but not `leads`.
pub struct Cell {
    pub driven: Option<bool>,
}
