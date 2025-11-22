{
  lib,
  installShellFiles,
  makeBinaryWrapper,
  makeRustPlatform,
  rev ? "dirty",
  wireguard-tools,
  rust-bin,
}:
let
  runtimeDeps = [
    wireguard-tools
  ];
  cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
  toolchain = rust-bin.selectLatestNightlyWith (toolchain: toolchain.default);
  rustPlatform = makeRustPlatform {
    rustc = toolchain;
    cargo = toolchain;
  };
in
rustPlatform.buildRustPackage {
  pname = "wgg";
  version = "${cargoToml.workspace.package.version}-${rev}";
  inherit toolchain;

  # src = lib.fileset.toSource {
  #   root = ./.;
  #   fileset = lib.fileset.intersection (lib.fileset.fromSource (lib.sources.cleanSource ./.)) (
  #     lib.fileset.unions [
  #       ./src
  #       ./Cargo.toml
  #       ./Cargo.lock
  #     ]
  #   );
  # };
  src = ./.;

  strictDeps = true;

  nativeBuildInputs = [
    installShellFiles
    makeBinaryWrapper
  ];

  postInstall = ''
    mkdir completions

    for shell in bash zsh fish; do
      $out/bin/wgg --completions $shell > completions/wgg.$shell
    done

    installShellCompletion completions/*

  '';

  postFixup = ''
    wrapProgram $out/bin/wgg \
      --prefix PATH : ${lib.makeBinPath runtimeDeps}
  '';

  cargoLock.lockFile = ./Cargo.lock;

  meta = {
    description = "human readable peer names for wireguard";
    homepage = "https://github.com/haennes/wg-friendly-peer-names";
    license = lib.licenses.mit;
    mainProgram = "wgg";
  };
}
