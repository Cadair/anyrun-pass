{
  description = "A plugin for anyrun for the pass password manager";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs = inputs: inputs.flake-parts.lib.mkFlake { inherit inputs; } {
    systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
    perSystem = { pkgs, ... }: let
      anyrun-pass = pkgs.rustPlatform.buildRustPackage {
        name = "anyrun-pass";
        src = builtins.path {
          path = inputs.self;
          name = "anyrun-pass";
        };
        cargoLock = {
          lockFile = ./Cargo.lock;
          # Temporary while packages aren't yet stabilized
          allowBuiltinFetchGit = true;
        };
        strictDeps = true;
        copyLibs = true;
        buildInputs = with pkgs; [ nettle openssl ];
        nativeBuildInputs = with pkgs; [
          pkg-config
          llvmPackages.clang
          llvmPackages.libclang
        ];
        LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
        CARGO_BUILD_INCREMENTAL = "false";
        RUST_BACKTRACE = "full";
      };
    in {

      packages.default = anyrun-pass;

      devShells.default = pkgs.mkShell {
        packages = with pkgs; [ rustc cargo anyrun ];
        shellHook = ''
          echo "Welcome to the anyrun-pass dev shell!"
          export ANYRUN_PASS_LIB="${anyrun-pass}/lib/libanyrun_pass.so"
          echo "Library path: $ANYRUN_PASS_LIB"
          alias anyrun-pass="anyrun --plugins $ANYRUN_PASS_LIB"
          echo "The anyrun-pass alias will run anyrun with the plugin"
        '';
      };
    };
  };
}
