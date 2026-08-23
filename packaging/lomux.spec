Name:           lomux
Version:        1.2.0
Release:        1%{?dist}
Summary:        Lightweight media converter with FFmpeg and yt-dlp integration

License:        MIT
URL:            https://github.com/zblauser/LoMux
Source0:        https://github.com/zblauser/LoMux/archive/v%{version}.tar.gz

BuildRequires:  rust >= 1.70
BuildRequires:  cargo
BuildRequires:  gtk3-devel
Requires:       ffmpeg
Suggests:       yt-dlp

%description
LoMux converts media files using FFmpeg with a clean, modern GUI.
Download from YouTube, convert between formats, edit metadata,
and use professional encoding presets — all in a tiny native binary.

%prep
%autosetup -n LoMux-%{version}

%build
cargo build --release --locked

%install
install -Dm755 target/release/lomux %{buildroot}%{_bindir}/lomux

%files
%license LICENSE
%{_bindir}/lomux

%changelog
* %(date "+%a %b %d %Y") zblauser <zblauser@users.noreply.github.com> - %{version}-1
- Initial RPM release
