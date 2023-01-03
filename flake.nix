{
  description = "The CloudMQTT Rust library";
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-22.05";
    flake-utils = {
      url = "github:numtide/flake-utils";
    };
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
  };

  outputs = { self, nixpkgs, crane, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        rustTarget = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        unstableRustTarget = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default.override {
          extensions = [ "rust-src" "miri" ];
        });
        craneLib = (crane.mkLib pkgs).overrideToolchain rustTarget;

        tomlInfo = craneLib.crateNameFromCargoToml { cargoToml = ./Cargo.toml; };
        inherit (tomlInfo) pname version;
        src = ./.;

        cargoArtifacts = craneLib.buildDepsOnly {
          inherit src;
        };

        cloudmqtt = craneLib.buildPackage {
          inherit cargoArtifacts src version;
          cargoExtraArgs = "--all-features";
        };

      in
      rec {
        checks = {
          inherit cloudmqtt;

          cloudmqtt-clippy = craneLib.cargoClippy {
            inherit cargoArtifacts src;
            cargoExtraArgs = "--all --all-features --all-targets";
            cargoClippyExtraArgs = "-- --deny warnings";
          };

          cloudmqtt-fmt = craneLib.cargoFmt {
            inherit src;
          };
        };

        packages.cloudmqtt = cloudmqtt;
        packages.default = packages.cloudmqtt;

        apps.cloudmqtt = flake-utils.lib.mkApp {
          name = "cloudmqtt";
          drv = cloudmqtt;
        };
        apps.default = apps.cloudmqtt;

        devShells.default = devShells.cloudmqtt;
        devShells.cloudmqtt = pkgs.mkShell {
          buildInputs = [
          ];

          nativeBuildInputs = [
            rustTarget
            #unstableRustTarget

            pkgs.cargo-msrv
            pkgs.cargo-deny
            pkgs.cargo-expand
            pkgs.cargo-bloat
            pkgs.cargo-fuzz

            pkgs.gitlint
          ];
        };
      }
    );
}
