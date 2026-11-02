# Non-flake entry point:  nix-build   |   nix-env -f . -i
{ pkgs ? import <nixpkgs> { } }:
pkgs.callPackage ./nix/package.nix { }
