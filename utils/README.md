# keel-utils

Rust replacements for the utilities shipped in Alpine's `musl-utils`
package. The package installs one multicall binary, `/usr/bin/keel-utils`,
and symlinks for the individual commands:

```text
getconf  getent  iconv  ldd  ldconfig
```

Invoke it directly to print the command list, or run one command through
the multicall interface (`keel-utils --help` prints the same list to stdout):

```sh
keel-utils            # usage summary on stderr; exits 1
keel-utils getconf PAGESIZE
keel-utils ldd /bin/sh
```

When invoked through one of the symlinks it behaves like the corresponding
command, matching the `musl-utils` interface.

## License

[BSD-3-Clause](../LICENSE); musl-derived parts are covered by the
MIT notice in [NOTICE](../NOTICE).
