{ dockerTools, pkgs, pkg, cmd, name }:

dockerTools.buildImage {
  inherit name;
  tag = "latest";

  copyToRoot = pkgs.buildEnv {
    name = "${name}-root";
    paths = [ pkg ];
    pathsToLink = [ "/bin" ];
  };

  config = {
    Cmd = [ cmd ];
  };
}
