<!--
@Time    : 2026/11/2 09:05
@Author  : fzf
@FileName: THIRD_PARTY_NOTICES.md
@Software: resource-pulse
-->

# Third-party components

Pulse's original code and artwork are licensed under Apache-2.0. Dependencies
retain their own licenses, copyright notices and trademarks.

The application uses GPUI and platform crates from
[zed-industries/zed](https://github.com/zed-industries/zed/tree/c4273271092b0287f7d181f616bd5171e0a2f70c),
not the Zed editor. Other direct dependencies include sysinfo, async-channel,
serde, serde_json, image, ksni, zbus, raw-window-handle, futures-lite and x11rb.
Exact versions and Git revisions are recorded in `Cargo.lock`.

Debian packages include `LICENSE`, this document and
`THIRD_PARTY_LICENSES.txt` under `/usr/share/doc/resource-pulse/`.
The generated inventory conservatively includes Cargo's resolved dependency
set, which can also include build tools, tests and other platform dependencies.
It records declared license expressions, source locations and the license and
notice files available in dependency source packages. Inclusion does not mean
every listed component is linked into the Linux binary.

Unmodified dependency source is available through the exact crates.io versions
and Git revisions in that inventory and `Cargo.lock`, including any MPL-2.0
components. Pulse does not change those upstream source files. System libraries
loaded dynamically are distributed by the operating system and retain the
licenses supplied by their respective packages.
