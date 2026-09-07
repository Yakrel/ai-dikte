Name:           ai-dikte
Version:        0.4.0
Release:        6%{?dist}
Summary:        Wayland dictation using Gemini Transcribe Live on KDE Plasma
License:        MIT
URL:            https://github.com/Yakrel/ai-dikte
# The workflow creates this archive from the checked-out repository, including PR changes.
Source0:        %{name}-%{version}.tar.gz
BuildArch:      noarch

BuildRequires:  python3-devel
Requires:       coreutils
Requires:       kwtype
Requires:       libnotify
Requires:       pipewire-utils
Requires:       python3
Requires:       python3-tkinter
Requires:       python3-websockets
Requires:       xorg-x11-server-Xwayland
Requires:       hicolor-icon-theme

%description
AI Dikte records microphone audio with PipeWire, transcribes it with Gemini,
and types the result directly into the focused application using KWtype on
KDE Plasma Wayland. Includes a shared Tk settings dialog and Meta+Z shortcut.

%prep
%autosetup

%install
install -Dm755 ai-dikte %{buildroot}%{_bindir}/ai-dikte
install -Dm755 ai-dikte-toggle %{buildroot}%{_bindir}/ai-dikte-toggle
# The launcher resolves ../lib/ai-dikte; this Python application is not lib64 data.
for module in ai_dikte.py ai_dikte_core.py ai_dikte_config.py ai_dikte_ui.py; do
    install -Dm644 "$module" "%{buildroot}%{_prefix}/lib/ai-dikte/$module"
done
install -Dm644 ai-dikte.png %{buildroot}%{_prefix}/lib/ai-dikte/ai-dikte.png
install -Dm644 ai-dikte.png %{buildroot}%{_datadir}/icons/hicolor/256x256/apps/ai-dikte.png
install -Dm644 ai-dikte-settings.desktop %{buildroot}%{_datadir}/applications/ai-dikte-settings.desktop
install -Dm644 ai-dikte.desktop %{buildroot}%{_datadir}/applications/ai-dikte.desktop
install -Dm644 ai-dikte.desktop %{buildroot}%{_datadir}/kglobalaccel/ai-dikte.desktop

%files
%license LICENSE
%{_bindir}/ai-dikte
%{_bindir}/ai-dikte-toggle
%{_prefix}/lib/ai-dikte/
%{_datadir}/icons/hicolor/256x256/apps/ai-dikte.png
%{_datadir}/applications/ai-dikte-settings.desktop
%{_datadir}/applications/ai-dikte.desktop
%dir %{_datadir}/kglobalaccel
%{_datadir}/kglobalaccel/ai-dikte.desktop
