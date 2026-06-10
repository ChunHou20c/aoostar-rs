{
  description = "A template for pinned nix channel";

  inputs = {
    unstable.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "unstable";
      };
    };
  };

  outputs =
    {
      self,
      unstable,
      flake-utils,
      rust-overlay,
    }:

    flake-utils.lib.eachDefaultSystem (
      system:
      let

        overlays = [ (import rust-overlay) ];
        overlay_pkgs = import unstable { inherit system overlays; };

      in
      {

        devShells = rec {

          rust-stable = import ./rust_stable.nix { pkgs = overlay_pkgs; };
          default = rust-stable;

        };
      }
    );
}
