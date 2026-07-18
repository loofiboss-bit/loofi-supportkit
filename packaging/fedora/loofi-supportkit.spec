Name:           loofi-supportkit
Version:        0.1.0
Release:        1%{?dist}
Summary:        Local-first system diagnostics and support cases

License:        MIT
URL:            https://github.com/loofiboss-bit/loofi-supportkit
Source0:        %{url}/archive/refs/tags/v%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  desktop-file-utils
BuildRequires:  gcc
BuildRequires:  gcc-c++
BuildRequires:  git-core
BuildRequires:  libappstream-glib
BuildRequires:  libappindicator-gtk3-devel
BuildRequires:  libxdo-devel
BuildRequires:  make
BuildRequires:  nodejs-npm
BuildRequires:  openssl-devel
BuildRequires:  rust
BuildRequires:  webkit2gtk4.1-devel

%description
Loofi SupportKit collects bounded, read-only system evidence, evaluates
deterministic rules, lets the user review privacy transformations, and exports
portable offline support cases. It does not repair systems, require an account,
upload data, or use AI in its trusted diagnostic core.

%prep
%autosetup

%build
npm ci --ignore-scripts --no-audit --no-fund
npm run tauri --workspace @loofi/supportkit-desktop -- build --no-bundle

%install
install -Dpm0755 target/release/loofi-supportkit-desktop \
  %{buildroot}%{_libexecdir}/loofi-supportkit/loofi-supportkit-desktop
install -Dpm0644 packaging/fedora/dev.loofi.supportkit.desktop \
  %{buildroot}%{_datadir}/applications/dev.loofi.supportkit.desktop
install -Dpm0644 packaging/fedora/dev.loofi.supportkit.metainfo.xml \
  %{buildroot}%{_metainfodir}/dev.loofi.supportkit.metainfo.xml
install -Dpm0644 apps/desktop/src-tauri/icons/128x128.png \
  %{buildroot}%{_datadir}/icons/hicolor/128x128/apps/dev.loofi.supportkit.png

%check
desktop-file-validate %{buildroot}%{_datadir}/applications/dev.loofi.supportkit.desktop
appstream-util validate-relax --nonet %{buildroot}%{_metainfodir}/dev.loofi.supportkit.metainfo.xml
cargo test --workspace --locked --release
npm test
npm run validate-schemas

%files
%license LICENSE
%doc README.md PRIVACY.md SECURITY.md docs/USER_GUIDE.md
%{_libexecdir}/loofi-supportkit/
%{_datadir}/applications/dev.loofi.supportkit.desktop
%{_metainfodir}/dev.loofi.supportkit.metainfo.xml
%{_datadir}/icons/hicolor/128x128/apps/dev.loofi.supportkit.png

%changelog
* Sat Jul 18 2026 Loofi SupportKit maintainers <noreply@users.noreply.github.com> - 0.1.0-1
- Initial public release.
