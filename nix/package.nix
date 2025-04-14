{
  rustPlatform,
  lib,
  libpulseaudio,
  pkg-config,
}:

let
  cargoToml = builtins.fromTOML (builtins.readFile ../Cargo.toml);
in
rustPlatform.buildRustPackage {
  pname = "panotify";
  inherit (cargoToml.package) version;

  cargoLock.lockFile = ../Cargo.lock;

  src = ../.;

  nativeBuildInputs = [ pkg-config ];

  buildInputs = [ libpulseaudio ];

  buildPhase = ''
    cargo build --release --workspace
  '';

  configurePhase = ''
    export PKG_CONFIG_PATH=${libpulseaudio.dev}/lib/pkgconfig
  '';

  installPhase = ''
    mkdir -p $out/bin
    cp target/release/panotify $out/bin/
  '';

  doCheck = false;

  meta = with lib; {
    description = "Pulse Audio and Notification bridge";
    homepage = "https://github.com/unixpariah/PAnotify";
    license = licenses.mit;
    maintainers = [ maintainers.unixpariah ];
    platforms = platforms.linux;
    mainProgram = "PAnotify";
  };
}
