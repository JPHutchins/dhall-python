{
  pkgs ?
    import
      (fetchTarball {
        url = "https://github.com/NixOS/nixpkgs/archive/ac62194c3917d5f474c1a844b6fd6da2db95077d.tar.gz";
        sha256 = "0v6bd1xk8a2aal83karlvc853x44dg1n4nk08jg3dajqyy0s98np";
      })
      { },
}:

pkgs.mkShell {
  buildInputs = with pkgs; [
    uv
    rustc
    cargo
    pkg-config
    openssl
  ];
  packages = with pkgs; [
    python310Packages.pytest
    dhall
    dhall-json
    rust-analyzer
  ];
}
