{
  pkgs ? import <nixpkgs> { },
}:
pkgs.mkShell {

  buildInputs = with pkgs; [
    openssl
    systemd
    pkg-config
    eza
    fd
    rust-bin.beta.latest.default
  ];

  shellHook = ''
    export SHELL="${pkgs.bash}/bin/bash"
    alias ls=eza
    alias find=fd
    echo "entering rust default stable build chain shell"
  '';
}
