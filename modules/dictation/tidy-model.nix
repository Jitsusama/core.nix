# Qwen2.5 1.5B Instruct, Alibaba's small instruct model (Apache 2.0), as
# OpenVINO exported it with its weights in int4: the model, its tokenizer
# and detokenizer, and the configs GenAI's LLM pipeline reads, at the
# revision whose tidies were measured. Dictation tidies a take with it
# (src/tidy.rs), where the account asks for that (home.nix).
#
# Only data is fetched, each file by its hash: the IRs and the JSON, never
# code. The tokenizers' own library is nixpkgs', which the service names.
# The template GenAI puts a prompt in is ours, not the export's: tidy.rs
# writes the whole prompt in the model's chat format, so the template only
# passes it through, and nothing from the model's repository is rendered.
{
  fetchurl,
  linkFarm,
  runCommandLocal,
}:
let
  revision = "4d14c299e35d8b74e3471f9e92bd1377fae50736";
  fetch =
    name: hash:
    fetchurl {
      url = "https://huggingface.co/OpenVINO/Qwen2.5-1.5B-Instruct-int4-ov/resolve/${revision}/${name}";
      inherit hash;
    };
  file = name: hash: {
    inherit name;
    path = fetch name hash;
  };
  template = "{% for message in messages %}{{ message['content'] }}{% endfor %}";
  # A tokenizer's IR with our template in place of the export's, which the
  # build fails on finding anywhere but where it was.
  templated = name: hash: {
    inherit name;
    path = runCommandLocal name { inherit template; } ''
      sed -E "s#<chat_template value=\"[^\"]*\"#<chat_template value=\"$template\"#" \
        ${fetch name hash} > $out
      if [ "$(grep -c '<chat_template value=' $out)" != 1 ] \
        || ! grep -qF "<chat_template value=\"$template\"" $out; then
        echo "$name's chat template isn't ours alone"
        exit 1
      fi
    '';
  };
  files = [
    (file "openvino_model.xml" "sha256-GmO9S+eudW5XgLpKo7aGvvJN2VOi4mjgqGiCURRih6U=")
    (file "openvino_model.bin" "sha256-wGoMc0bmytjeoM3z8OEfT109vEn6eTZrhn0Xy6ehxz0=")
    (templated "openvino_tokenizer.xml" "sha256-2fs6orJEN0PqTZAci6ECrpEEk3JamVz9QVP4gVkieAU=")
    (file "openvino_tokenizer.bin" "sha256-A8MkxFqRXIfgPAlbc5xWc3Hl5eExvxOyBSrJ5lm1gsw=")
    (templated "openvino_detokenizer.xml" "sha256-Z8VRLkTILCPSnMKeyxszd/bYw4GLf6hWTpfQaWO6bcY=")
    (file "openvino_detokenizer.bin" "sha256-PKR2AVVKO4ccHkXo8xqm4X1yY2XAdftDDRogiUhK6NY=")
    (file "config.json" "sha256-Av7D+gMETZmyXtsStZECqTAfNlvprm4JldRb4MShiO4=")
    (file "generation_config.json" "sha256-cioGCR7IccbLw92lQS8pQme81dkxD0/77x01gADdO/w=")
  ];
in
(linkFarm "qwen2.5-1.5b-instruct-int4-ov" files).overrideAttrs (previous: {
  # What it holds, by name, for the check that it's data alone.
  passthru = (previous.passthru or { }) // {
    files = map (file: file.name) files;
  };
})
