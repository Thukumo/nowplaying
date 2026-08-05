{
  description = "Self-hosted now playing server with a ListenBrainz-compatible API";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

  outputs =
    { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
      server = pkgs.callPackage ./nix/package.nix {
        rustPlatform = pkgs.pkgsStatic.rustPlatform;
        pname = "nowplaying";
        bin = "nowplaying";
      };
      client = pkgs.callPackage ./nix/package.nix {
        pname = "nowplaying-client";
        bin = "nowplaying-client";
        nativeBuildInputs = [ pkgs.pkg-config ];
        buildInputs = [ pkgs.dbus ];
      };
      dashboard = pkgs.callPackage ./nix/package.nix {
        rustPlatform = pkgs.pkgsStatic.rustPlatform;
        pname = "nowplaying-dashboard";
        bin = "nowplaying-dashboard";
      };
    in
    {
      packages.${system} = {
        default = server;
        server = server;
        client = client;
        dashboard = dashboard;
        image = pkgs.callPackage ./nix/image.nix {
          pkg = server;
          cmd = "/bin/nowplaying";
          name = "nowplaying";
        };
        image-dashboard = pkgs.callPackage ./nix/image.nix {
          pkg = dashboard;
          cmd = "/bin/nowplaying-dashboard";
          name = "nowplaying-dashboard";
        };
      };

      devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [
          cargo
          rustc
          rustfmt
          clippy
          rust-analyzer
          pkg-config
          dbus
        ];
      };

      formatter.${system} = nixpkgs.legacyPackages.${system}.nixfmt-tree;
    };
}
