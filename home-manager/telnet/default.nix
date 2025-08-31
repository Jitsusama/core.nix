{ pkgs, ... }:
{
  home.packages = [
    (pkgs.symlinkJoin {
      name = "telnet";
      paths = [ pkgs.inetutils ];
      postBuild = ''
        find $out/bin -type l -not -name telnet -delete
        find $out/share/man -type f -not -name 'telnet*' -delete 2>/dev/null || true
        find $out/share/man -type d -empty -delete 2>/dev/null || true
      '';
    })
  ];
}

