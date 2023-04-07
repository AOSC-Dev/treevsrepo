treevsrepo / 照妖镜
===================

A simple tool to expose package version discrepancies between the
[aosc-os-abbs](https://github.com/AOSC-Dev/aosc-os-abbs) tree and the
[community repository](https://repo.aosc.io).

Usage
-----

```
USAGE:
    treevsrepo [OPTIONS] --tree <TREE>

OPTIONS:
    -a, --arch <ARCH>...     Set search arch
    -h, --help               Print help information
    -m, --mirror <MIRROR>    Set mirror [default: https://repo.aosc.io]
    -o, --output <OUTPUT>    Output result to file
    -r, --retro              Set branch (retro/non-retro)
    -t, --tree <TREE>        Set tree directory. e.g: /home/saki/aosc-os-abbs
    -V, --version            Print version information
```

For instance, to check for version discrepancies for the `amd64` architecture:

```bash
treevsrepo -a amd64 -t /path/to/tree
```

To output a list of packages with version discrepancies:

```bash
treevsrepo -a amd64 -t /path/to/tree -o groups/version-diff
```

Installing or Building
----------------------

This tool is available from the AOSC OS community repository as the `treevsrepo`
package. Install via the following command:

```bash
sudo apt install treevsrepo
```

Or, you may build this package with a Rust toolchain:

```bash
cargo build --release
```

And the resulting binaries should be found in the `./target/release` directory.

