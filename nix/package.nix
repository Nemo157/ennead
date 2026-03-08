{ lib, rustPlatform, pkg-config, udev }:

rustPlatform.buildRustPackage {
  pname = "ἐννεάς-cli";
  version = "0.1.0";

  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../cli
      ../protocol
    ];
  };

  cargoRoot = "cli";
  buildAndTestSubdir = "cli";

  cargoLock = {
    lockFile = ../cli/Cargo.lock;
    outputHashes = {
      "dither-1.3.10" = "sha256-KrdCmLJbsXaRnmzB0tAS5tg9+8krvoUMZFwb7QgqO7M=";
    };
  };

  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ udev ];

  meta.mainProgram = "ἐννεάς-cli";
}
