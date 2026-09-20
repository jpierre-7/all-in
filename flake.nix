{
  description = "all-in — Bevy game dev shell";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};

      # Loaded at runtime via dlopen by winit/wgpu, so they are never probed
      # at build time and must be on LD_LIBRARY_PATH instead of buildInputs.
      runtimeLibs = with pkgs; [
        wayland
        libxkbcommon
        vulkan-loader
        libx11
        libxcursor
        libxrandr
        libxi
      ];
    in
    {
      devShells.${system}.default = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [
          pkg-config

          # Pinned here rather than taken from the host, so the toolchain is
          # part of the locked environment like everything else. Cargo.lock
          # pins dependency crates; it does not pin the compiler.
          rustc
          cargo
          clippy
          rustfmt
          rust-analyzer
        ];

        # Exactly the libraries our -sys crates pkg-config-probe:
        # alsa-sys -> alsa, libudev-sys -> libudev, wayland-sys -> wayland-*.
        # .github/workflows/ci.yml apt-installs the same list -- keep them in sync.
        buildInputs = with pkgs; [
          alsa-lib
          udev
          wayland
        ];

        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath runtimeLibs;

        # Lets rust-analyzer find the standard library sources.
        RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
      };
    };
}
