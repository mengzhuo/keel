# keel Docker image

`docker/Dockerfile` builds an Alpine 3.23 image whose system libc and ELF
interpreter are **keel** instead of musl. It is meant for testing and for
running musl-linked images against keel, not for replacing the package
manager's libc in production.

## Build

```bash
# x86_64 (or the host architecture)
docker build -f docker/Dockerfile -t keel:0.8.0 .

# Multi-arch (requires buildx and a registry/loading backend)
docker buildx build \
  --platform linux/amd64,linux/arm64,linux/riscv64 \
  -f docker/Dockerfile -t keel:0.8.0 .
```

The image downloads the keel release APK from GitHub. To use a local mirror
(or a package that is already on an internal HTTP server):

```bash
docker build -f docker/Dockerfile \
  --build-arg KEEL_VERSION=0.8.0 \
  --build-arg KEEL_APK_URL=http://mirror.example/keel-0.8.0-r0.x86_64.apk \
  --build-arg KEEL_UTILS_URL=http://mirror.example/keel-utils-0.8.0-r0.x86_64.apk \
  -t keel:0.8.0 .
```

Behind an HTTP proxy:

```bash
docker build --network host \
  --build-arg http_proxy=http://proxy:3128 \
  --build-arg https_proxy=http://proxy:3128 \
  -f docker/Dockerfile -t keel:0.8.0 .
```

## Run

```bash
docker run --rm -it keel:0.8.0 sh
docker run --rm keel:0.8.0 /bin/echo hello
```

At runtime, `LD_LIBRARY_PATH=/usr/lib/keel` is set so DSOs that record
`libc.so` (rather than the musl SONAME) also resolve to keel.

## What the image does

1. Installs the `keel` and `keel-utils` APKs.
2. `keel` provides `musl` and `so:libc.musl-<arch>.so.1`, so installing it
   automatically purges Alpine's `musl` package.
3. `keel` installs the canonical paths as symlinks:
   - `/lib/libc.musl-<arch>.so.1 -> /usr/lib/keel/libc.so`
   - `/lib/ld-musl-<arch>.so.1 -> /usr/lib/keel/libldso.so`
4. `keel-utils` provides `musl-utils`, so installing it automatically purges
   `musl-utils` and `scanelf`, and installs Rust `getconf`, `getent`, `iconv`,
   `ldd` and `ldconfig`.
5. Smoke-tests `/bin/echo`, `/bin/sh`, `apk --version`, `getconf` and `ldd`.

## Caveats

- The image installs `keel-utils`, not Alpine's `musl-utils` commands.
  `keel-utils` provides `musl-utils`/`libc-utils`/`cmd:*`, so installing it
  automatically purges `musl-utils` and `scanelf`.  It is a static Rust
  multicall binary.
- `keel-dev` is not installed in the runtime image. Install it explicitly
  if you want to compile inside the container; it provides the full
  `musl-dev` header set, `libc.a` and Rust-built CRT objects. It also
  provides `musl-dev`, so `apk add build-base` continues to work and
  automatically replaces Alpine's `musl-dev`. Static PIE, static non-PIE,
  constructors and initial-exec TLS are supported.
- `/usr/bin/keel-loader` is the standalone mapper for static PIE/static ELF
  binaries, not a replacement for the dynamic interpreter. The image uses
  `/lib/ld-musl-<arch>.so.1` (keel `libldso.so`) for normal dynamic binaries.
- The stock musl files are replaced in place; the `keel` package itself
  declares `provides: musl` and `replaces: musl`, and installing it purges
  the `musl` package. Use `apk fix musl` only if you intend to switch back.
- Installing `keel-dev` purges Alpine's `musl-dev`; packages that depend on
  `musl-dev` are satisfied by `keel-dev`'s `provides`, so `build-base` and
  `g++` keep working.
- `dlclose` follows musl: it validates the handle but does not unload modules
  or run destructors. Build a normal musl image if glibc-style unloading is
  required.
- The image is architecture-specific. `TARGETARCH` is mapped to Alpine's
  `x86_64` / `aarch64` / `riscv64` names. The x86_64 image is verified
  locally; build aarch64/riscv64 on a native runner (or with a working
  buildx/binfmt setup) because qemu-user emulation can trip over Alpine's
  busybox shell independently of keel.
