// A stand-in for mycelium's source, which core's mycelium checks read
// (carry-guards.nix): with `driven`, so only easing.rs's commented `leads`
// can fail it.
pub struct Cell {
    pub driven: Option<bool>,
}
