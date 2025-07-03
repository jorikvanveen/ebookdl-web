{
  inputs = { utils.url = "github:numtide/flake-utils"; };
  outputs = { self, nixpkgs, utils }:
    utils.lib.eachDefaultSystem (system:
      let pkgs = nixpkgs.legacyPackages.${system};
      in {
        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [ cargo rustc rust-analyzer libgourou ];
        };
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "ebookdl";
          version = "0.1.0";

          src = ./.;
          cargoLock = { lockFile = ./Cargo.lock; };
        };
      });
}
