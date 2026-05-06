{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = inputs:
    inputs.flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = (import (inputs.nixpkgs) { inherit system; });
      in {
        devShell = pkgs.mkShell {
          shellHook = ''
              fish -C "source prompt.fish"
              exit
            '';
          buildInputs = with pkgs; [
            fish
            qt5.qtbase
            qt5.wrapQtAppsHook
            pkg-config
          ];
        };
      }
    );
}
