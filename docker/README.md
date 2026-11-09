# crabc Docker image

`docker/Dockerfile` builds an Alpine 3.23 image whose system libc and ELF
interpreter are **crabc** instead of musl. It is meant for testing and for
running musl-linked images against crabc, not for replacing the package
manager's libc in production.

## Build

```bash
# x86_64 (or the host architecture)
docker build -f docker/Dockerfile -t crabc:0.7.4 .

# Multi-arch (requires buildx and a registry/loading backend)
docker buildx build \
  --platform linux/amd64,linux/arm64,linux/riscv64 \
  -f docker/Dockerfile -t crabc:0.7.4 .
```

The image downloads the crabc release APK from GitHub. To use a local mirror
(or a package that is already on an internal HTTP server):

```bash
docker build -f docker/Dockerfile \
  --build-arg CRABC_VERSION=0.7.4 \
  --build-arg CRABC_APK_URL=http://mirror.example/crabc-0.7.4-r0.x86_64.apk \
  --build-arg CRABC_UTILS_URL=http://mirror.example/crabc-utils-0.7.4-r0.x86_64.apk \
  -t crabc:0.7.4 .
```

Behind an HTTP proxy:

```bash
docker build --network host \
  --build-arg http_proxy=http://proxy:3128 \
  --build-arg https_proxy=http://proxy:3128 \
  -f docker/Dockerfile -t crabc:0.7.4 .
```

## Run

```bash
docker run --rm -it crabc:0.7.4 sh
docker run --rm crabc:0.7.4 /bin/echo hello
```

At runtime, `LD_LIBRARY_PATH=/usr/lib/crabc` is set so DSOs that record
`libc.so` (rather than the musl SONAME) also resolve to crabc.

## What the image does

1. Installs the `crabc` and `crabc-utils` APKs.
2. `crabc` provides `musl` and `so:libc.musl-<arch>.so.1`, so installing it
   automatically purges Alpine's `musl` package.
3. `crabc` installs the canonical paths as symlinks:
   - `/lib/libc.musl-<arch>.so.1 -> /usr/lib/crabc/libc.so`
   - `/lib/ld-musl-<arch>.so.1 -> /usr/lib/crabc/libldso.so`
4. `crabc-utils` provides `musl-utils`, so installing it automatically purges
   `musl-utils` and `scanelf`, and installs Rust `getconf`, `getent`, `iconv`,
   `ldd` and `ldconfig`.
5. Smoke-tests `/bin/echo`, `/bin/sh`, `apk --version`, `getconf` and `ldd`.

## Caveats

- The image installs `crabc-utils`, not Alpine's `musl-utils` commands.
  `crabc-utils` provides `musl-utils`/`libc-utils`/`cmd:*`, so installing it
  automatically purges `musl-utils` and `scanelf`.  It is a static Rust
  multicall binary.
- `crabc-dev` is not installed in the runtime image. Install it explicitly
  if you want to compile inside the container; it provides the full
  `musl-dev` header set, `libc.a` and Rust-built CRT objects. It also
  provides `musl-dev`, so `apk add build-base` continues to work and
  automatically replaces Alpine's `musl-dev`. Static PIE, static non-PIE,
  constructors and initial-exec TLS are supported.
- `/usr/bin/crabc-loader` is the standalone mapper for static PIE/static ELF
  binaries, not a replacement for the dynamic interpreter. The image uses
  `/lib/ld-musl-<arch>.so.1` (crabc `libldso.so`) for normal dynamic binaries.
- The stock musl files are replaced in place; the `crabc` package itself
  declares `provides: musl` and `replaces: musl`, and installing it purges
  the `musl` package. Use `apk fix musl` only if you intend to switch back.
- Installing `crabc-dev` purges Alpine's `musl-dev`; packages that depend on
  `musl-dev` are satisfied by `crabc-dev`'s `provides`, so `build-base` and
  `g++` keep working.
- `dlclose` follows musl: it validates the handle but does not unload modules
  or run destructors. Build a normal musl image if glibc-style unloading is
  required.
- The image is architecture-specific. `TARGETARCH` is mapped to Alpine's
  `x86_64` / `aarch64` / `riscv64` names. The x86_64 image is verified
  locally; build aarch64/riscv64 on a native runner (or with a working
  buildx/binfmt setup) because qemu-user emulation can trip over Alpine's
  busybox shell independently of crabc.
