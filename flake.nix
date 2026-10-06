{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { nixpkgs, rust-overlay, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };
      rustToolchain = pkgs.rust-bin.stable."1.99.0".minimal.override {
        extensions = [ "rustfmt" "clippy" ];
      };
    in {
      devShells.${system}.default = pkgs.mkShell {
        nativeBuildInputs = [ pkgs.rustPlatform.bindgenHook ];
        buildInputs = [ pkgs.curl pkgs.openssl pkgs.libxml2 ];
        packages = [
          rustToolchain
          pkgs.cargo-deny
          pkgs.just
          pkgs.hurl
          pkgs.pkg-config
        ];
        RUSTC = "${rustToolchain}/bin/rustc";
        RUSTDOC = "${rustToolchain}/bin/rustdoc";
        shellHook = ''
          # Keep user-installed subcommands behind the shell's pinned tools.
          export PATH="$PATH:''${CARGO_HOME:-$HOME/.cargo}/bin"

          # Keep Nix artifacts separate from other toolchains' builds.
          export CARGO_TARGET_DIR="''${CARGO_TARGET_DIR:-$PWD/target/nix}"
          export CARGO_BUILD_JOBS="''${CARGO_BUILD_JOBS:-2}"
        '';
        CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER = "${pkgs.stdenv.cc}/bin/cc";
      };
    };
}
