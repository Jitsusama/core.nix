# How a live local site is built: its TypeScript typechecked at its
# strictest and its tests run as it builds, so a site that doesn't hold
# never reaches the server, then compiled to the browser's modules beside
# its page. The kit (kit/) is what every site shares: a watch of a daemon's
# feed, the records it holds, the line saying how live it is, and the theme
# switch. It is built once and mounted at /local-sites/kit/, where every
# site's compiled imports already point, with the theme's web kit beside it
# at /local-sites/web/, where every page reads its styles and fonts.
#
# That holds because of where a site lives. Its source is at
# modules/<name>/site/ and imports the kit as ../../local-sites/kit/<x>.ts,
# which the compiler rewrites to .js; it is served at /<name>/, one folder
# deep, where that address climbs to the root and lands on the kit's mount.
# A site served deeper would need its imports rewritten, which nothing here
# does; one laid out to stand alone has them rewritten by `standalone`.
{ lib, pkgs }:
let
  tsc = lib.getExe' pkgs.typescript "tsc";
  node = lib.getExe pkgs.nodejs;
  # What writes a sheet again without its comments (site.sh says why).
  esbuild = lib.getExe pkgs.esbuild;

  # The kit, typechecked and tested, as the browser's modules and the
  # switch's styles, without their comments as a site's are.
  kit = pkgs.runCommandLocal "local-sites-kit" { } ''
    cp -r ${./kit} kit
    chmod -R u+w kit
    cd kit
    ${tsc} -p tsconfig.json
    ${node} --test --test-reporter=spec tests/*.test.ts
    ${tsc} -p tsconfig.json --noEmit false --outDir $out/kit
    ${esbuild} --log-level=warning switch.css > $out/kit/switch.css 2> said
    if [ -s said ]; then cat said >&2; exit 1; fi
  '';
in
{
  inherit kit;

  /**
    What every live site shares, mounted at /local-sites/: the kit, and the
    theme's web kit `webKit` beside it as web/, where each page reads
    theme.css, base.css and the fonts. One copy at one address, rather than
    one in every site, so the browser fetches the fonts once for them all
    and holds them once. Each is linked rather than copied, so a file keeps
    the real path of its own build, which the server's tag names: a change
    to the kit leaves the fonts' tags as they were, and the browser keeps
    what it holds of them.
  */
  shared =
    webKit:
    pkgs.runCommandLocal "local-sites-shared" { } ''
      mkdir $out
      ln -s ${kit}/kit $out/kit
      ln -s ${webKit} $out/web
    '';

  /**
    The site `name`, from its source `src` (modules/<name>/site), its page
    reading the theme's web kit from what the sites share, at
    ../local-sites/web/. Each look's background and the role `tint` the
    site marks its emphasis in fill the page's theme-color and its favicon
    (site.sh says how), from `looks`, each look's roles, which default to
    those of the theme's web kit `webKit`.
  */
  localSite =
    {
      name,
      src,
      webKit,
      tint ? "action",
      looks ? webKit.looks,
    }:
    # The site carries the web kit it was built on, so a folder laid out
    # from it copies the same one in.
    pkgs.runCommandLocal "${name}-site" { passthru = { inherit webKit; }; } ''
      ${lib.getExe pkgs.bash} ${./site.sh} ${
        lib.escapeShellArgs [
          tsc
          node
          esbuild
          "${./kit}"
          "${src}"
          name
        ]
      } "$out" ${
        lib.escapeShellArgs [
          looks.dark.background
          looks.light.background
          looks.dark.${tint}
          looks.light.${tint}
        ]
      }
    '';

  /**
    The live site `site`, built by localSite as `name`, as one folder that
    stands alone, for a host that serves a folder as it is: its page at the
    root, the kit and the web kit it was built on copied in as kit/ and
    web/, and every address its page and modules load relative and within
    the folder, or the build fails (standalone.sh says how). The strip
    goes, since the home it leads to stays here. Its feed is still asked of
    /feed/<name>, so on a host that bridges none the page says the sites'
    server can't be reached and keeps trying, rather than drawing nothing;
    on a host that answers there, what answers is what it draws, as text,
    the commands it offers to copy among them, so a folder belongs only on
    a host whose /feed/ the reader trusts. How a folder reaches a host
    stays outside core.
  */
  standalone =
    { name, site }:
    pkgs.runCommandLocal "${name}-standalone" { } ''
      ${lib.getExe pkgs.bash} ${./standalone.sh} ${
        lib.escapeShellArgs [
          "${site}"
          "${kit}/kit"
          "${site.webKit}"
        ]
      } "$out"
    '';
}
