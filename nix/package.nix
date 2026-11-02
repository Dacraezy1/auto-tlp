{ lib, rustPlatform, installShellFiles }:

rustPlatform.buildRustPackage {
  pname = "auto-tlp";
  version = (builtins.fromTOML (builtins.readFile ../Cargo.toml)).package.version;

  src = lib.cleanSource ../.;
  cargoLock.lockFile = ../Cargo.lock;

  nativeBuildInputs = [ installShellFiles ];

  postInstall = ''
    installManPage man/auto-tlp.1
    installShellCompletion --bash --name auto-tlp completions/auto-tlp.bash
    installShellCompletion --zsh  --name _auto-tlp completions/_auto-tlp
    installShellCompletion --fish --name auto-tlp.fish completions/auto-tlp.fish
  '';

  meta = with lib; {
    description = "Scans your hardware and generates a clean, tailored TLP configuration";
    homepage = "https://github.com/Dacraezy1/auto-tlp";
    license = licenses.gpl3Plus;
    platforms = platforms.linux;
    mainProgram = "auto-tlp";
  };
}
