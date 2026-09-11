%global commit ac2c3864aaacc31afc252d88d1d4b669270f2f44
%global source_sha256 ec6f1fa5128835dabbbb3f819ba5aea318eb6e81a8ec47d9fb2868af152c7b41

Name:           kwtype
Version:        0.1.0
Release:        1%{?dist}
Summary:        Direct keyboard input on KDE Plasma Wayland
License:        MIT
URL:            https://github.com/Sporif/KWtype
Source0:        https://github.com/Sporif/KWtype/archive/%{commit}.tar.gz

BuildRequires:  coreutils
BuildRequires:  gcc-c++
BuildRequires:  meson
BuildRequires:  ninja-build
BuildRequires:  pkgconf-pkg-config
# Fedora's Qt 6 kwayland-devel supplies KWaylandClient.pc, not kf5-kwayland-devel.
BuildRequires:  pkgconfig(KWaylandClient)
BuildRequires:  pkgconfig(Qt6Core)
BuildRequires:  pkgconfig(Qt6DBus)
BuildRequires:  pkgconfig(wayland-client)
BuildRequires:  pkgconfig(xkbcommon)

%description
KWtype sends synthetic keyboard events through KDE Plasma's Wayland fake-input
protocol. It is the native KDE typing backend used by AI Dikte.

%prep
# Verify the raw upstream archive before extracting or executing any source.
printf '%%s  %%s\n' '%{source_sha256}' '%{SOURCE0}' | sha256sum --check --strict -
%autosetup -n KWtype-%{commit}

%build
%meson
%meson_build

%install
%meson_install

%files
%license LICENSE
%{_bindir}/kwtype
%{_datadir}/applications/kwtype.desktop
