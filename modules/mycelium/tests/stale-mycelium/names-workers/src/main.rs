// A stand-in for mycelium's source, which core's mycelium checks read
// (eases.nix): after #441, declaring the flag that names the workers its
// node may ease onto.
struct EasesAsked {
    eases_onto: Vec<String>,
}
