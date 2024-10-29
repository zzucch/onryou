{
  inputs = {
    fenix.url = "github:nix-community/fenix";
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };
  outputs = {
    self,
    fenix,
    flake-utils,
    naersk,
    nixpkgs,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = (import nixpkgs) {
          inherit system;
        };
        toolchain = with fenix.packages.${system};
          combine [
            minimal.rustc
            minimal.cargo
            targets.x86_64-pc-windows-gnu.latest.rust-std
          ];
        naerskWindows = naersk.lib.${system}.override {
          cargo = toolchain;
          rustc = toolchain;
        };
        naersk' = pkgs.callPackage naersk {};
      in {
        defaultPackage = naersk'.buildPackage {
          src = ./.;
        };
        packages.x86_64-pc-windows-gnu = naerskWindows.buildPackage {
          src = ./.;
          strictDeps = true;
          depsBuildBuild = with pkgs; [
            pkgsCross.mingwW64.stdenv.cc
            pkgsCross.mingwW64.windows.pthreads
          ];
          CARGO_BUILD_TARGET = "x86_64-pc-windows-gnu";
        };
        devShell = with pkgs;
          mkShell {
            buildInputs = [
              cargo
              rustc
              rustfmt
              rustPackages.clippy

              openssl
              pkg-config
            ];
          };
      }
    );
}
