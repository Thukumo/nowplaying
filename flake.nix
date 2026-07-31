{
  description = "Self-hosted now playing server with a ListenBrainz-compatible API";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/e8273b29fe1390ec8d4603f2477357555291432e";

  outputs =
    { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
      server = pkgs.callPackage ./nix/package.nix {
        pname = "nowplaying";
        bin = "nowplaying";
      };
      client = pkgs.callPackage ./nix/package.nix {
        pname = "nowplaying-client";
        bin = "nowplaying-client";
      };
    in
    {
      packages.${system} = {
        default = server;
        server = server;
        client = client;
        image = pkgs.callPackage ./nix/image.nix { inherit server; };
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
    };
}
