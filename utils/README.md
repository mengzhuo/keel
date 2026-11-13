# keel-utils

Rust replacements for the utilities shipped in Alpine's `musl-utils`
package. The package installs one multicall binary, `/usr/bin/keel-utils`,
and symlinks for the individual commands:

```text
getconf  getent  iconv  ldd  ldconfig
```

Invoke it directly for a summary:

```sh
keel-utils
keel-utils getconf PAGESIZE
keel-utils ldd /bin/sh
```

When invoked through one of the symlinks it behaves like the corresponding
command, matching the `musl-utils` interface.

## License

[BSD-3-Clause](../LICENSE)
