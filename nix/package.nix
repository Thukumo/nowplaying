{
  rustPlatform,
  lib,
  pname,
  bin,
  nativeBuildInputs ? [ ],
  buildInputs ? [ ],
}:

rustPlatform.buildRustPackage (rec {
  inherit pname nativeBuildInputs buildInputs;
  version = "0.1.0";

  src = lib.cleanSource ../.;
  cargoLock.lockFile = ../Cargo.lock;

  binFlags = [
    "--bin"
    bin
  ];
  cargoBuildFlags = binFlags;
  cargoTestFlags = binFlags;
  cargoInstallFlags = binFlags;

  meta = {
    description = "nowplaying ${bin}";
    license = lib.licenses.mit;
    mainProgram = bin;
  };
})
