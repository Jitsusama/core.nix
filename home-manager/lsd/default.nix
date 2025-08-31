{
  ...
}:
{
  programs.lsd = {
    enable = true;
    settings = {
      blocks = [
        "permission"
        "user"
        "group"
        "size"
        "date"
        "git"
        "name"
      ];
      date = "+%Y-%m-%dT%H:%M:%S";
    };
  };
}
