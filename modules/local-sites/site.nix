# How a live local site is built: its TypeScript typechecked at its
# strictest and its tests run as it builds, so a site that doesn't hold
# never reaches the server, then compiled to the browser's modules beside
# its page. The kit (kit/) is what every site shares: a watch of a daemon's
# feed, the records it holds, the line saying how live it is, and the theme
# switch. It is built once and mounted at /local-sites/kit/, where every
# site's compiled imports already point.
#
# That holds because of where a site lives. Its source is at
# modules/<name>/site/ and imports the kit as ../../local-sites/kit/<x>.ts,
# which the compiler rewrites to .js; it is served at /<name>/, one folder
# deep, where that address climbs to the root and lands on the kit's mount.
# A site elsewhere, or served deeper, would need its imports rewritten,
# which nothing here does.
{ lib, pkgs }:
let
  tsc = lib.getExe' pkgs.typescript "tsc";
  node = lib.getExe pkgs.nodejs;
  # A site's files the browser reads as they are: its page, its styles, its
  # icon, and its Markdown and plain-text twins.
  static = [
    "html"
    "css"
    "svg"
    "md"
    "txt"
  ];
  copyStatic =
    from:
    lib.concatMapStringsSep "\n" (
      ext: "find ${from} -maxdepth 1 -name '*.${ext}' -exec cp {} $out/ \\;"
    ) static;
in
{
  # The kit, typechecked and tested, as the browser's modules and the
  # switch's styles.
  kit = pkgs.runCommandLocal "local-sites-kit" { } ''
    cp -r ${./kit} kit
    chmod -R u+w kit
    cd kit
    ${tsc} -p tsconfig.json
    ${node} --test --test-reporter=spec tests/*.test.ts
    ${tsc} -p tsconfig.json --noEmit false --outDir $out/kit
    cp switch.css $out/kit/
  '';

  /**
    The site `name`, from its source `src` (modules/<name>/site), with the
    theme's web kit `webKit` copied into its kit/ folder, where its page
    reads theme.css, base.css and the fonts. Copied rather than linked,
    since the server follows no link out of what it serves.
  */
  localSite =
    {
      name,
      src,
      webKit,
    }:
    pkgs.runCommandLocal "${name}-site" { } ''
      # The sources as they sit in the tree, so the site's imports of the
      # kit resolve here as they do there.
      mkdir -p work/modules/local-sites work/modules/${name}
      cp -r ${./kit} work/modules/local-sites/kit
      cp -r ${src} work/modules/${name}/site
      chmod -R u+w work
      cd work/modules/${name}/site
      ${tsc} -p tsconfig.json
      if [ -d tests ]; then
        ${node} --test --test-reporter=spec tests/*.test.ts
      fi
      ${tsc} -p tsconfig.json --noEmit false --rootDir ../.. --outDir "$TMPDIR/emitted"
      mkdir -p $out
      cp "$TMPDIR"/emitted/${name}/site/*.js $out/
      ${copyStatic "."}
      cp -r ${webKit} $out/kit
      chmod -R u+w $out/kit
    '';
}
