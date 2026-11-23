// A stand-in for mycelium's source, which core's mycelium checks read
// (carry-guards.nix): with `leads` but not `driven`.
pub struct Cell {
    pub fence: Option<Kind>,
}
