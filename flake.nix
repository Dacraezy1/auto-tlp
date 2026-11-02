{
  description = "auto-tlp - generate a hardware-specific TLP configuration";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAll = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAll (pkgs: rec {
        auto-tlp = pkgs.callPackage ./nix/package.nix { };
        default = auto-tlp;
      });

      apps = forAll (pkgs: rec {
        auto-tlp = {
          type = "app";
          program = "${self.packages.${pkgs.system}.auto-tlp}/bin/auto-tlp";
        };
        default = auto-tlp;
      });

      overlays.default = final: _prev: {
        auto-tlp = final.callPackage ./nix/package.nix { };
      };

      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [ cargo rustc rustfmt clippy rust-analyzer shellcheck ];
        };
      });
    };
}
