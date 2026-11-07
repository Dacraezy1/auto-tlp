# Source RPM spec for Fedora / openSUSE / RHEL-likes (COPR, OBS, local rpmbuild).
# Pre-built, static .rpm files are also attached to every GitHub release.
%global debug_package %{nil}

Name:           auto-tlp
Version:        0.2.0
Release:        1%{?dist}
Summary:        Scans your hardware and generates a tailored TLP configuration
License:        GPL-3.0-or-later
URL:            https://github.com/Dacraezy1/auto-tlp
Source0:        %{url}/archive/v%{version}/%{name}-%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  gcc
BuildRequires:  make
Requires:       tlp

%description
auto-tlp scans your machine (CPU and its frequency driver, battery, NVMe/SATA/HDD
storage, GPUs, Wi-Fi, USB, audio, ...) and writes a TLP configuration containing
only the settings your hardware actually supports.

%prep
%autosetup

%build
cargo build --release --locked

%check
cargo test --release --locked

%install
make install DESTDIR=%{buildroot} PREFIX=%{_prefix} BIN=target/release/auto-tlp

%files
%license LICENSE
%doc README.md
%{_bindir}/auto-tlp
%{_mandir}/man1/auto-tlp.1*
%{_datadir}/bash-completion/completions/auto-tlp
%{_datadir}/zsh/site-functions/_auto-tlp
%{_datadir}/fish/vendor_completions.d/auto-tlp.fish
%{_datadir}/licenses/auto-tlp/LICENSE

%changelog
* Mon Nov 02 2026 Dacraezy1 <dacraezy1@users.noreply.github.com> - 0.1.0-1
- Initial package
