# crabc Docker image

`docker/Dockerfile` builds an Alpine 3.23 image whose system libc and ELF
interpreter are **crabc** instead of musl. It is meant for testing and for
running musl-linked images against crabc, not for replacing the package
manager's libc in production.

## Build

```bash
# x86_64 (or the host architecture)
docker build -f docker/Dockerfile -t crabc:0.7.2 .

# Multi-arch (requires buildx and a registry/loading backend)
docker buildx build \
  --platform linux/amd64,linux/arm64,linux/riscv64 \
  -f docker/Dockerfile -t crabc:0.7.2 .
```

The image downloads the crabc release APK from GitHub. To use a local mirror
(or a package that is already on an internal HTTP server):

```bash
docker build -f docker/Dockerfile \
  --build-arg CRABC_VERSION=0.7.2 \
  --build-arg CRABC_APK_URL=http://mirror.example/crabc-0.7.2-r0.x86_64.apk \
  --build-arg CRABC_UTILS_URL=http://mirror.example/crabc-utils-0.7.2-r0.x86_64.apk \
  -t crabc:0.7.2 .
```

Behind an HTTP proxy:

```bash
docker build --network host \
  --build-arg http_proxy=http://proxy:3128 \
  --build-arg https_proxy=http://proxy:3128 \
  -f docker/Dockerfile -t crabc:0.7.2 .
```

## Run

```bash
docker run --rm -it crabc:0.7.2 sh
docker run --rm crabc:0.7.2 /bin/echo hello
```

At runtime, `LD_LIBRARY_PATH=/usr/lib/crabc` is set so DSOs that record
`libc.so` (rather than the musl SONAME) also resolve to crabc.

## What the image does

1. Installs the `crabc` and `crabc-utils` APKs.
2. Replaces the stock Alpine musl libc/loader in place.
3. Places the crabc files at the canonical paths:
   - `/lib/libc.musl-<arch>.so.1`
   - `/lib/ld-musl-<arch>.so.1`
4. Leaves symlinks in `/usr/lib/crabc` pointing at the canonical paths.
5. Installs Rust `getconf`, `getent`, `iconv`, `ldd` and `ldconfig` from
   `crabc-utils` (replacing the base image's `musl-utils` commands).
6. Smoke-tests `/bin/echo`, `/bin/sh`, `apk --version`, `getconf` and `ldd`.

## Caveats

- The image installs `crabc-utils`, not Alpine's `musl-utils` commands. The
  APK database still records `musl-utils`; `apk fix musl-utils` would restore
  its `ldd`/`getconf` files. `crabc-utils` is a Rust multicall binary.
- `crabc-dev` is not installed in the runtime image. Install it explicitly
  if you want to compile inside the container; it provides headers,
  `libc.a` and Rust-built CRT objects. Static PIE is not supported; use
  `-no-pie` for static programs.
- `/usr/bin/crabc-loader` is the standalone mapper for static PIE/static ELF
  binaries, not a replacement for the dynamic interpreter. The image uses
  `/lib/ld-musl-<arch>.so.1` (crabc `libldso.so`) for normal dynamic binaries.
- The stock musl files are replaced in place; use the `musl` APK
  (`apk fix musl`) if you need to switch back. Do not run
  `apk upgrade musl` if the image should stay on crabc.
- The APK database still records the `musl` package. Package operations that
  verify or reinstall musl can overwrite the crabc files.
- `dlclose` follows musl: it validates the handle but does not unload modules
  or run destructors. Build a normal musl image if glibc-style unloading is
  required.
- The image is architecture-specific. `TARGETARCH` is mapped to Alpine's
  `x86_64` / `aarch64` / `riscv64` names. The x86_64 image is verified
  locally; build aarch64/riscv64 on a native runner (or with a working
  buildx/binfmt setup) because qemu-user emulation can trip over Alpine's
  busybox shell independently of crabc.
