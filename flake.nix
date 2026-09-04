{
  description = "NEAR Intents contracts";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    cargo-near-src = {
      url = "github:near/cargo-near/cargo-near-v0.22.0";
      flake = false;
    };
  };

  outputs = { nixpkgs, rust-overlay, cargo-near-src, ... }:
    let
      supportedSystems = [
        "aarch64-darwin"
        "x86_64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
      pkgsFor = system: import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };
    in
    {
      devShells = forAllSystems (system:
        let
          pkgs = pkgsFor system;
          rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain;
          cargo-near = pkgs.rustPlatform.buildRustPackage {
            pname = "cargo-near";
            version = "0.22.0";
            src = cargo-near-src;
            cargoLock = {
              lockFile = "${cargo-near-src}/Cargo.lock";
            };
            nativeBuildInputs = pkgs.lib.optionals pkgs.stdenv.isLinux [ pkgs.pkg-config ];
            buildInputs = pkgs.lib.optionals pkgs.stdenv.isLinux [ pkgs.openssl pkgs.udev ];
          };
        in
        {
          default = pkgs.mkShellNoCC {
            packages = with pkgs; [
              rustToolchain
              cargo-near
              taplo
              cargo-machete
              cargo-audit
              jq
              nodejs_22
              cosign
              gh
              gnumake
            ] ++ lib.optionals stdenv.isLinux [
              docker_29
              pkg-config
              openssl
              udev
            ];
          };
        });
    };
}
