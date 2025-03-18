{ pkgs, craneLib }:

let
  inherit (pkgs) lib;
  src = craneLib.cleanCargoSource ../.;
  commonArgs = {
    inherit src;
    strictDeps = true;
  };
  cargoArtifacts = craneLib.buildDepsOnly commonArgs;

  individualCrateArgs = commonArgs // {
    inherit cargoArtifacts;
    inherit (craneLib.crateNameFromCargoToml { inherit src; }) version;

    doCheck = false;
  };

  fileSetForCrate =
    crate:
    lib.fileset.toSource {
      root = ../.;
      fileset = lib.fileset.unions [
        ../Cargo.lock
        ../Cargo.toml
        (craneLib.fileset.commonCargoSources ../crates/hakari)
        (craneLib.fileset.commonCargoSources ../crates/mqtt-format)
        (craneLib.fileset.commonCargoSources crate)
      ];
    };
in
{
  packages = {
    cloudmqtt = craneLib.buildPackage (
      individualCrateArgs
      // {
        pname = "cloudmqtt";
        cargoExtraARgs = "-p cloudmqtt";
        src = fileSetForCrate ../crates/cloudmqtt;
      }
    );
  };
  checks = {
    workspace-clippy = craneLib.cargoClippy (
      commonArgs
      // {
        inherit cargoArtifacts;
        cargoClippyExtraArgs = "--all-targets -- --deny warnings";
      }
    );

    workspace-doc = craneLib.cargoDoc (
      commonArgs
      // {
        inherit cargoArtifacts;
      }
    );

    workspace-fmt = craneLib.cargoFmt ({
      inherit src;
    });

    workspace-nextest = craneLib.cargoNextest (
      commonArgs
      // {
        inherit cargoArtifacts;
        partitions = 1;
        partitionType = "count";
        cargoNextestPartitionsExtraArgs = "--no-tests=pass";
      }
    );

    workspace-hakari = craneLib.mkCargoDerivation {
      inherit src;
      pname = "workspace-hakari";
      cargoArtifacts = null;
      doInstallCargoArtifacts = false;

      buildPhaseCargoCommand = ''
        cargo hakari generate --diff  # workspace-hack Cargo.toml is up-to-date
        cargo hakari manage-deps --dry-run  # all workspace crates depend on workspace-hack
        cargo hakari verify
      '';

      nativeBuildInputs = [
        pkgs.cargo-hakari
      ];
    };
  };
}
