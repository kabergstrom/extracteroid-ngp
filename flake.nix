
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
    nixpkgs-unstable.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    flake-utils.url = "github:numtide/flake-utils";

    # wild = {
    #   url = "github:davidlattimore/wild";
    #   inputs.nixpkgs.follows = "nixpkgs";
    # };
  };

  nixConfig = {
    extra-substituters = [
      "https://nix-community.cachix.org"
    ];
    extra-trusted-public-keys = [
      "nix-community.cachix.org-1:mB9FSh9Uf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
    ];
  };

  outputs = { self, nixpkgs, nixpkgs-unstable, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };

        pkgsUnstable = import nixpkgs-unstable {
          inherit system;
        };

        isDarwin = pkgs.stdenv.isDarwin;
        isLinux = pkgs.stdenv.isLinux;

        # A list of libraries needed specifically at runtime for dynamic linking (Linux only).
        runtimeLibs = pkgs.lib.optionals isLinux (with pkgs; [
          libxkbcommon
          vulkan-loader
          xorg.libX11
          xorg.libXcursor
          xorg.libXrandr
          xorg.libXi
        ]);

      in
      {
        packages.default = pkgs.callPackage ./package.nix { };

        devShells.default = pkgsUnstable.mkShell {
          buildInputs = pkgs.lib.optionals isLinux (with pkgs; [
              vulkan-validation-layers
          ]) ++ pkgs.lib.optionals isDarwin [
              pkgsUnstable.apple-sdk_15
          ];
          packages = [
            # Dioxus CLI
            pkgsUnstable.dioxus-cli

            # Rust toolchain
            pkgsUnstable.cargo
            pkgsUnstable.rustc
            pkgsUnstable.rust-analyzer
            pkgsUnstable.cargo-watch
            pkgsUnstable.pkg-config

            # Build tools
            # pkgs.wild
            pkgsUnstable.cmake
            pkgsUnstable.git
            pkgsUnstable.python3
          ] ++ pkgs.lib.optionals isLinux (with pkgs; [
            # Vulkan support (Linux; macOS uses LunarG Vulkan SDK)
            vulkan-headers
            vulkan-validation-layers
            vulkan-tools
            glslang
          ]) ++ runtimeLibs;

          # THE FIX: Manually construct and set the library path.
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath runtimeLibs;

          RUST_SRC_PATH = "${pkgsUnstable.rustPlatform.rustLibSrc}";

          shellHook = pkgs.lib.optionalString isDarwin ''
            # Source the most recent local Vulkan SDK
            if [ -d "$HOME/VulkanSDK" ]; then
              _vulkan_setup="$(ls -1d "$HOME"/VulkanSDK/*/setup-env.sh 2>/dev/null | sort -V | tail -1)"
              if [ -n "$_vulkan_setup" ]; then
                source "$_vulkan_setup"
              fi
              unset _vulkan_setup
            fi
          '';

          env = {
            RUST_BACKTRACE = "full";
          } // pkgs.lib.optionalAttrs isLinux {
            VK_LAYER_PATH = "${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d";
          };
        };
      });
}
