{ rustPlatform, lib }:

rustPlatform.buildRustPackage {
  pname = "nowplaying";
  version = "0.1.0";

  src = lib.cleanSource ../.;
  cargoLock.lockFile = ../Cargo.lock;

  meta = {
    description = "Self-hosted now playing server (ListenBrainz-compatible API)";
    license = lib.licenses.mit;
    mainProgram = "nowplaying";
  };
}
