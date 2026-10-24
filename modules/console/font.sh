# shellcheck shell=bash disable=SC2154 # The face, glyphs and converter are the build's attributes in nixos.nix.
# Builds the console's font from the face fontconfig finds, held to the face
# and the Medium style asked for, since fontconfig answers with its nearest
# when they aren't there. The face found is written beside the font.

export HOME=$TMPDIR
found=$(fc-match --format '%{family}\n%{style}\n%{file}' "$face:style=Medium")
family=$(echo "$found" | sed -n 1p)
style=$(echo "$found" | sed -n 2p)
file=$(echo "$found" | sed -n 3p)
case ",$family," in
  *,"$face",*) ;;
  *)
    echo "$face isn't among the machine's fonts (fonts.packages); fontconfig offered $family" >&2
    exit 1
    ;;
esac
case ",$style," in
  *,Medium,*) ;;
  *)
    echo "$face has no Medium; fontconfig offered $style" >&2
    exit 1
    ;;
esac
mkdir -p "$out"
"$convert" "$file" "$glyphs" "$out/console.psf"
printf '%s\n%s\n' "$face" Medium > "$out/face"
