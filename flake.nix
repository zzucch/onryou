{
  inputs = {
    fenix.url = "github:nix-community/fenix";
    naersk.url = "github:nix-community/naersk";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };
  outputs = {
    self,
    fenix,
    naersk,
    nixpkgs,
    ...
  } @ inputs: let
    supportedSystems = [
      "x86_64-linux"
      "aarch64-linux"
      "x86_64-darwin"
      "aarch64-darwin"
    ];
    forEachSupportedSystem = f:
      nixpkgs.lib.genAttrs supportedSystems (
        system: let
          pkgs = import inputs.nixpkgs {inherit system;};
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
        in
          f pkgs naersk' naerskWindows
      );
  in {
    packages = forEachSupportedSystem (pkgs: naersk: naerskWindows: {
      default = naersk.buildPackage {
        src = ./.;
      };
      x86_64-pc-windows-gnu = naerskWindows.buildPackage {
        src = ./.;
        strictDeps = true;
        depsBuildBuild = with pkgs; [
          pkgsCross.mingwW64.stdenv.cc
          pkgsCross.mingwW64.windows.pthreads
        ];
        CARGO_BUILD_TARGET = "x86_64-pc-windows-gnu";
      };
    });
    devShells = forEachSupportedSystem (pkgs: _: _: {
      default = pkgs.mkShell {
        packages = with pkgs; [
          cargo
          rustc
          rustfmt
          rustPackages.clippy

          openssl
          pkg-config

          alejandra
        ];
      };
    });
  };
}
