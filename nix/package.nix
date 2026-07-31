{ rustPlatform, lib, pkg-config, dbus, pname, bin }:

rustPlatform.buildRustPackage {
  inherit pname;
  version = "0.1.0";

  src = lib.cleanSource ../.;
  cargoLock.lockFile = ../Cargo.lock;

  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ dbus ];

  cargoBuildFlags = [ "--bin" bin ];
  cargoInstallFlags = [ "--bin" bin ];

  meta = {
    description = "nowplaying ${bin}";
    license = lib.licenses.mit;
    mainProgram = bin;
  };
}
