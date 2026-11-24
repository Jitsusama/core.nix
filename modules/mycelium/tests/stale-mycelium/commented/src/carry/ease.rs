// A stand-in for mycelium's source, which core's mycelium checks read
// (carry-guards.nix): naming `driven` only in comments.
pub struct Cell {
    // pub driven: Option<bool>,
    /// pub driven: Option<bool>,
    pub fence: Option<Kind>,
}
