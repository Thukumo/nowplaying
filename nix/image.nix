{ dockerTools, pkgs, package }:

dockerTools.buildImage {
  name = "nowplaying";
  tag = "latest";

  copyToRoot = pkgs.buildEnv {
    name = "nowplaying-root";
    paths = [ package ];
    pathsToLink = [ "/bin" ];
  };

  config = {
    Cmd = [ "/bin/nowplaying" ];
    ExposedPorts = {
      "8080/tcp" = { };
    };
  };
}