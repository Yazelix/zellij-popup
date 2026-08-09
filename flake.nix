{
  description = "Zellij plugin for toggling configured floating TUI popups";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix.url = "github:nix-community/fenix";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      fenix,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };
        rustToolchain = fenix.packages.${system}.combine [
          fenix.packages.${system}.stable.cargo
          fenix.packages.${system}.stable.rustc
          fenix.packages.${system}.targets.wasm32-wasip1.stable.rust-std
        ];
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };
        zellijPluginWasmPackageContract = {
          schemaVersion = 1;
          pluginName = "yazelix-zellij-popup";
          packageAttr = "yazelix_zellij_popup";
          wasmPath = "share/yazelix_zellij_popup/yzpp.wasm";
          wasmTarget = "wasm32-wasip1";
          cargoAuditableDisabled = true;
          cargoBuildHookDisabled = true;
          preBuildPreservesNixRustToolchain = true;
          wasmTargetRustcEnvPinned = true;
          cargoBuildSerialized = true;
          installCheckVerifiesWasm = true;
        };
        yazelixZellijPopup = rustPlatform.buildRustPackage {
          pname = "yazelix-zellij-popup";
          version = "0.1.0";
          src = pkgs.lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;
          auditable = !zellijPluginWasmPackageContract.cargoAuditableDisabled;
          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = [ pkgs.openssl ];
          dontCargoBuild = zellijPluginWasmPackageContract.cargoBuildHookDisabled;
          doCheck = false;

          buildPhase = ''
            yazelix_saved_cargo="''${CARGO:-$(command -v cargo || true)}"
            yazelix_saved_rustc="''${RUSTC:-$(command -v rustc || true)}"
            yazelix_saved_path="$PATH"
            if [ -z "$yazelix_saved_cargo" ] || [ -z "$yazelix_saved_rustc" ]; then
              echo "Nix Rust hooks did not provide cargo/rustc before preBuild" >&2
              exit 1
            fi

            runHook preBuild

            export CARGO="$yazelix_saved_cargo"
            export RUSTC="$yazelix_saved_rustc"
            export PATH="$yazelix_saved_path"
            export CARGO_BUILD_RUSTC="$RUSTC"
            export CARGO_TARGET_WASM32_WASIP1_RUSTC="$RUSTC"

            wasm_target_libdir="$("$RUSTC" --print target-libdir --target ${zellijPluginWasmPackageContract.wasmTarget})"
            if [ ! -d "$wasm_target_libdir" ]; then
              echo "Rust toolchain is missing wasm32-wasip1 std at $wasm_target_libdir" >&2
              exit 1
            fi

            "$CARGO" build \
              -j 1 \
              --target-dir target \
              --offline \
              --profile release \
              --target ${zellijPluginWasmPackageContract.wasmTarget}

            runHook postBuild
          '';

          installPhase = ''
            runHook preInstall

            install -Dm644 target/wasm32-wasip1/release/yzpp.wasm \
              "$out/${zellijPluginWasmPackageContract.wasmPath}"
            mkdir -p "$out/share/yazelix_zellij_popup/examples"
            substitute examples/gitui.kdl \
              "$out/share/yazelix_zellij_popup/examples/gitui.kdl" \
              --replace-fail "__YZPP_WASM__" \
              "file:$out/${zellijPluginWasmPackageContract.wasmPath}"
            install -Dm644 examples/gitui.kdl \
              "$out/share/yazelix_zellij_popup/examples/gitui.template.kdl"
            install -Dm644 README.md "$out/share/doc/yazelix_zellij_popup/README.md"

            runHook postInstall
          '';

          doInstallCheck = true;
          nativeInstallCheckInputs = [
            pkgs.coreutils
            pkgs.gnugrep
          ];
          installCheckPhase = ''
            runHook preInstallCheck

            test -s "$out/${zellijPluginWasmPackageContract.wasmPath}"
            grep -q 'location="file:' "$out/share/yazelix_zellij_popup/examples/gitui.kdl"
            grep -q 'MessagePlugin "yzpp"' "$out/share/yazelix_zellij_popup/examples/gitui.kdl"
            grep -q 'command "gitui"' "$out/share/yazelix_zellij_popup/examples/gitui.kdl"
            grep -q 'name "toggle"' "$out/share/yazelix_zellij_popup/examples/gitui.kdl"
            ! grep -q 'gitui_command' "$out/share/yazelix_zellij_popup/examples/gitui.kdl"
            ! grep -q '__YZPP_WASM__' "$out/share/yazelix_zellij_popup/examples/gitui.kdl"

            runHook postInstallCheck
          '';

          passthru = {
            inherit zellijPluginWasmPackageContract;
            wasmPath = zellijPluginWasmPackageContract.wasmPath;
            examplePath = "share/yazelix_zellij_popup/examples/gitui.kdl";
            templatePath = "share/yazelix_zellij_popup/examples/gitui.template.kdl";
          };

          meta = {
            description = "Zellij plugin for toggling configured floating TUI popups";
            homepage = "https://github.com/Yazelix/zellij-popup";
            license = pkgs.lib.licenses.asl20;
          };
        };
      in
      {
        packages = {
          default = yazelixZellijPopup;
          yazelix-zellij-popup = yazelixZellijPopup;
          yazelix_zellij_popup = yazelixZellijPopup;
          yzpp = yazelixZellijPopup;
        };

        devShells.default = pkgs.mkShell {
          packages = [
            rustToolchain
            pkgs.pkg-config
            pkgs.openssl
          ];
        };
      }
    );
}
