Name:           ai-dikte
Version:        0.5.0
Release:        1%{?dist}
Summary:        Rust voice dictation using Gemini Live on KDE Plasma
License:        MIT
URL:            https://github.com/Yakrel/ai-dikte
Source0:        %{name}-%{version}.tar.gz
# Dependency sources are fetched with cargo --locked before rpmbuild and built offline.
Source1:        vendor.tar.gz
BuildRequires:  rust >= 1.98
BuildRequires:  cargo
BuildRequires:  gcc
BuildRequires:  pkgconf-pkg-config
Requires:       kwtype
Requires:       libnotify
Requires:       pipewire-utils
Requires:       libxkbcommon
Requires:       libxkbcommon-x11
Requires:       libX11
Requires:       libXcursor
Requires:       libXi
Requires:       libXrandr
Requires:       libglvnd-egl
Requires:       libglvnd-glx
Requires:       wayland-libs
Requires:       hicolor-icon-theme

%description
AI Dikte records microphone audio with PipeWire, transcribes it with Gemini,
and types the result into the focused application using KWtype. Includes
native Rust settings and a user service for Meta+Z dictation.

%prep
%autosetup
%{__tar} -xzf %{SOURCE1}
mkdir -p rust/.cargo
cat > rust/.cargo/config.toml <<'CONFIG'
[source.crates-io]
replace-with = "vendored-sources"
[source.vendored-sources]
directory = "../vendor"
CONFIG

%build
cd rust
cargo build --frozen --release

%check
cd rust
cargo test --frozen --lib -- --skip live::tests
./target/release/ai-dikte --self-test

%install
DESTDIR=%{buildroot} AI_DIKTE_BINARY=rust/target/release/ai-dikte sh packaging/install-linux.sh

%files
%{_bindir}/ai-dikte
%{_bindir}/ai-dikte-toggle
%{_datadir}/icons/hicolor/256x256/apps/ai-dikte.png
%{_datadir}/applications/ai-dikte-settings.desktop
%{_datadir}/applications/ai-dikte.desktop
%dir %{_datadir}/kglobalaccel
%{_datadir}/kglobalaccel/ai-dikte.desktop
%{_prefix}/lib/systemd/user/ai-dikte.service
%license %{_datadir}/licenses/ai-dikte/LICENSE
