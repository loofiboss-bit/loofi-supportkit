# Fedora collector inventory

The runtime currently owns system profile, failed systemd units, DNF update state, disk/mount state, NetworkManager connection state, resolver configuration, PCI GPU data, and loaded modules. All process calls are fixed in `crates/collector-runtime`; this directory documents review scope and must not contain executable scripts.

Fedora support is limited to Fedora KDE 44 x64. No elevation is requested. Unavailable utilities create `skipped` records and do not abort the scan.
