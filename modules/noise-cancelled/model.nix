# DeepFilterNet 3, the denoiser behind the noise-cancelled microphones, as
# one ONNX graph taking a 10 ms frame and its state and giving the cleaned
# frame and the next state (export/export.py says how and checks it). It's
# exported from the checkpoint DeepFilterNet publishes in its repository,
# through grazder's torchDF, which holds the whole denoiser, its signal
# processing too, in PyTorch, since the ONNX DeepFilterNet publishes takes
# whole recordings and can't stream.
{
  fetchFromGitHub,
  git,
  python3,
  runCommand,
}:
let
  # grazder's fork, whose torchDF folder is the streaming model; its
  # models folder holds DeepFilterNet's own checkpoints.
  source = fetchFromGitHub {
    owner = "grazder";
    repo = "DeepFilterNet";
    rev = "1097015d53ced78fb234e7d7071a5dd4446e3952";
    hash = "sha256-A7CewppkFHVJzicEXSANgyVOiZoDCrSHF8pIaRm9Qp4=";
  };
  python = python3.withPackages (
    packages: with packages; [
      # What DeepFilterNet's model code imports.
      appdirs
      loguru
      numpy
      packaging
      requests
      sympy
      torch
      # What the export writes, and checks it with.
      onnx
      openvino
    ]
  );
in
runCommand "deepfilternet3-frame"
  {
    # DeepFilterNet's logger asks git for the commit it runs from, and
    # copes with git saying it isn't in a repository but not with no git.
    nativeBuildInputs = [
      git
      python
    ];
  }
  ''
    cp -r ${source}/torchDF torchDF
    chmod -R u+w torchDF
    # The check whether a norm's state is still unset, made one tracing
    # follows rather than one it bakes in as it finds it on the first frame.
    for state in erb_norm_state band_unit_norm_state; do
      substituteInPlace torchDF/torch_df_streaming.py --replace-fail \
        "torch.tensor(torch.nonzero($state).shape[0] == 0)" "torch.all($state == 0)"
    done
    python -c "import zipfile; zipfile.ZipFile('${source}/models/DeepFilterNet3.zip').extractall()"
    mkdir $out
    # A path, since the bare name DeepFilterNet3 is one DeepFilterNet
    # downloads rather than reads.
    HOME=$TMPDIR PYTHONPATH=${./export/stand-in}:${source}/DeepFilterNet:torchDF \
      python ${./export/export.py} "$PWD/DeepFilterNet3" $out/deepfilternet3-frame.onnx
  ''
