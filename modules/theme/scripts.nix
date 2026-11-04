# The script face: Noto, script by script, for the scripts the theme's own
# faces lack. Without it Hebrew and Arabic fell back to the machine's
# default sans, drawn for neither. The theme's font.script options default
# to these, and the machine's fonts install them, so their names live here
# alone.
{
  # What fontconfig falls back to, in turn, for a letter no face has. CJK's
  # Japanese cut, since its kana and kanji serve the Japanese calendar and
  # its Han serve the Chinese one.
  families = [
    "Noto Sans Hebrew"
    "Noto Naskh Arabic"
    "Noto Sans CJK JP"
  ];
  # The cut a surface names for words it knows are in one script, set beside
  # the serif face or the mono one. Qt falls back by script rather than by
  # language, so Chinese Han would take the Japanese cut's forms unless the
  # Chinese cut is named. Naskh is Arabic's own book hand, so it serves both.
  serif = {
    hebrew = "Noto Serif Hebrew";
    arabic = "Noto Naskh Arabic";
    chinese = "Noto Serif CJK SC";
    japanese = "Noto Serif CJK JP";
  };
  mono = {
    hebrew = "Noto Sans Hebrew";
    arabic = "Noto Naskh Arabic";
    chinese = "Noto Sans Mono CJK SC";
    japanese = "Noto Sans Mono CJK JP";
  };
}
