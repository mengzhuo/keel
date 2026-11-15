# keel 文档/注释审计报告

> 审计日期：2026-11-15
> 审计对象：`keel` 本地仓库，commit `132ffa3`
> 审计类型：Markdown 文档、脚本头注释、Rust/C 源码注释
> 不包含：安全漏洞审计（另见 `docs/2026-11-15_security-audit-keel-report.md`）

## 0. 执行摘要

文档整体质量较好：README、`docs/`、各 workload 报告、打包/容器说明和
`AGENTS.md` 覆盖了主要使用场景，源码注释也普遍解释了“为什么”而不只是
复述代码。

本轮发现的主要问题是**快照数据漂移**和**源码中的内部标记/过时注释**：

- 1 个 Markdown 链接锚点失效。
- 约 20 处版本、计数、架构范围或行为描述已过时/不准确。
- 50 处 `ponytail:` 内部标记，其中若干条把真实限制描述成“够测试用”。
- 多处注释与当前实现不符（C11 线程“stub”、`posix_spawn`“最多 5 个
  action”、`getpagesize`“仅 x86_64”、`DT_NEEDED`“任何真实二进制都够用”
  等）。
- 一个重要的文档缺口：stdio 锁、`random` 锁、`wcwidth`、页大小、
  `at_quick_exit`、`getpass`、独立 loader 环境变量、`DT_NEEDED` 上限等
  行为差异此前只散落在源码注释里，用户文档没有集中说明。

上述可直接修复的问题已在本轮修正，并新增 `docs/limitations.md` 汇总
当前已知行为缺口。

## 1. 检查方法

- 遍历所有 Markdown 文件，检查相对链接、锚点、代码围栏和标题结构。
- 对照 `Cargo.toml`、`scripts/`、`.github/workflows/`、`libc-test-harness/reports/`
  和实际构建产物，核对文档中的版本、计数、路径和命令。
- 逐文件阅读 Rust/C 头注释，重点检查 `TODO`/`FIXME`/“stub”/“for tests”/
  “simplified”/“currently” 等措辞。
- 运行 `cargo run -q -p keel-header-check -- --include include` 核对头文件
  数值宏（1407/1407 匹配）。
- 运行 `cargo check -p keel-libc -p keel-rt` 和
  `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps`，确认文档
  注释不会引入构建或 rustdoc 警告。

## 2. 已修复

### 2.1 Markdown 文档

| 文件 | 问题 | 修复 |
|---|---|---|
| `README.md` | “`no_std` libc and loader” 不准确：独立 `keel` mapper 使用 `std` | 区分 no_std 动态链接器与 std 独立 mapper |
| `README.md` | 未说明 nightly、`musl-gcc`、`file`、binutils 和静态 CRT 前置条件 | 增加构建/测试前置条件 |
| `README.md` | 声称 libc-test/LTP/workload 套件都覆盖三架构，与实际记录不符 | 改为 libc-test x86_64+aarch64，LTP/workload x86_64，riscv64 为 cross-build+qemu |
| `README.md` | 把用户指向 `AGENTS.md` 获取交叉编译说明 | 主指向 `docs/packages.md`，`AGENTS.md` 仅作 agent 参考 |
| `docs/comparison.md` | `../README.md#status-and-scope` 锚点不存在 | 改为 `#status` |
| `docs/comparison.md` | 只写 414 PASS / 5 SKIP，未说明 root 与 CI 的差异 | 补充 root 与非特权 CI 两组计数及日期 |
| `docs/performance.md` | 声称 `libc.so` 使用 `-z pack-relative-relocs`，与 `libc/build.rs` 和 `AGENTS.md` 矛盾 | 改为“loader 支持 DT_RELR，但 keel 自身仍用 RELA” |
| `docs/performance.md` | “current one is 1.06x” 是合并 loader 之前的包大小数据 | 删除过期比值，保留历史 2.3x 的背景 |
| `docs/performance.md` | “shrinks libc.so by about 30%” 未说明这是合并 loader 前的数据 | 改为 1.52 MB → 680 KB 的历史事实 |
| `docs/performance.md` | `libc.a` 写 9.7 MB，当前产物约 10.3 MB | 改为约 10 MB |
| `docs/packages.md` | 声称三架构都使用 self-contained musl sysroot | 改为“三目标均链接 Rust self-contained musl；riscv64 使用 rust-lld” |
| `docs/packages.md` | 安装大小 1.9/10/1.9 MiB 与当前产物不符 | 改为约 2.3/11/0.7 MiB |
| `docs/ltp.md` | Caveat 把 libc-test harness 的 `src/<subset>/` 行为误写成 LTP 行为 | 改为 LTP build/install 树共享、不要并发运行 |
| `docs/workloads.md` | 0.4.2 与当前 0.1.1 的版本关系容易误解 | 标明是 rename 前的 `crabc` 0.4.2，版本从 0.1.0 重新开始 |
| `docs/workloads.md` | BusyBox scope 写“304 applets”容易被理解为全部测试 | 改为“304 applets available；代表性集合” |
| `docs/workload-suites.md` | “40 modules”与“62 modules OK”看似矛盾 | 说明 CPython 会把 package 型测试展开为 62 个 test file |
| `docs/workload-suites.md` | “latest run 0.1.0”与当前 0.1.1 未区分 | 标注记录日期 2026-11-12 |
| `docs/workload-suites.md` | 未说明 suite 只在 x86_64 运行 | 在开头和结果处补充 x86_64 |
| `docs/workload-suites.md` | 发布门禁写成 test+macro-parity | 补上 `header-consistency` |
| `libc-test-harness/README.md` | functional 写 72/72，最新报告为 73/73 | 更新并注明计数随外部 libc-test checkout 变化 |
| `libc/README.md` | “~350+ C functions” 已过时；`f128` 支持未说明精度限制 | 改为 musl 1.2.6 public API，并注明 long-double 数学走 f64 |
| `utils/README.md` | “Invoke it directly for a summary” 实际是 stderr + exit 1 | 改为“打印命令列表（stderr，退出 1）”，并指向 `--help` |
| `AGENTS.md` | baseline 的 functional 72/72 已过时 | 改为 73/73，并注明 2026-11-15 复核 |

### 2.2 源码注释

- 删除全部 **50 处** `ponytail:` 内部标记，改成普通注释；其中不准确的
  表述单独重写。
- `libc/src/safety.rs`：原文称“本文件唯一的 unsafe 操作是构造 view”，
  与实际多个 unsafe 块不符；改为“unsafe 前置条件在构造时建立，后续
  unsafe 块由该不变量证明”。
- `libc/src/regression_stubs.rs`：文件名/头注释称“链接用 stub”，实际
  包含 `sysconf`、`execle`、`flockfile`、`strdup`、`mktemp` 等真实实现。
- `libc/src/lib.rs`：C11 线程“Thread function stubs - need actual
  implementation”已过时，改为 pthread 包装。
- `libc/src/lib.rs`：`posix_spawn_file_actions` 的“max 5 actions”已过时，
  实现是 growable linked list。
- `libc/src/lib.rs`：`getpagesize` 注释只提 x86_64，实际三架构都返回
  4096；改为明确说明该限制。
- `libc/src/lib.rs`：`random()` 注释“add lock if concurrency matters”
  淡化了与 musl 的差异；改为明确 musl 加锁、keel 当前不加锁。
- `libc/src/lib.rs`：`tsearch` 注释“correct for tests”改为“unbalanced
  BST，最坏 O(n)，与 musl 的 AVL 不同”。
- `libc/src/lib.rs`：`wcwidth`/`wcswidth` 注释明确“无 Unicode width
  tables，组合字符和双宽字符不正确”。
- `libc/src/lib.rs`：`at_quick_exit`、`getpass` 改为明确 TODO，不再用
  “simplified”掩盖。
- `libc/src/lib.rs`：`strcoll`/`wcscoll` 注释改为“与 musl 的 C locale
  字节/码点比较一致”，避免误读为 keel 自创的简化。
- `libc/src/ldso/mod.rs`：`DT_NEEDED` 注释改为明确 TODO（最多 32 个直接
  依赖，超出会丢弃），不再写“任何真实二进制都够用”。
- `libc/src/statvfs.rs`、`libc/src/lrand48.rs`、`libc/src/crypt_impl.rs`
  的 `ponytail` 注释改为准确描述。
- `src/main.rs`：补充说明独立 mapper 不转发宿主环境，目标得到空 `envp`。
- `src/arch.rs`：删除空的 “Architecture selection” 分节标题。
- `libc/src/lib.rs` 中 6 处 “musl x86_64 layout” 改为 “musl 64-bit layout”。

### 2.3 新增 `docs/limitations.md`

把此前只存在于源码注释中的行为差异集中到用户文档，包括：

- stdio 不自动加锁，且 `flockfile` 不会被 stdio 函数遵守；
- `random` 家族共享未加锁的全局状态；
- locale/Unicode width/iconv 编码集限制；
- `at_quick_exit`、`getpass`、4096 页大小、`fnmatch` 1024 字节上限、
  `tsearch` 非平衡树；
- 独立 loader 不传环境、主程序 `DT_NEEDED` 上限 32、`dlclose` 不卸载；
- x86_64 64-bit `long double`、aarch64/riscv64 long-double 数学走 `f64`；
- `vfork`/`posix_spawn` 使用 `fork` 语义。

## 3. 未修复/建议

### 3.1 许可证归属（建议优先处理）

仓库中至少 18 个 `libc/src/*.rs` 明确写着 “Translated from musl” 或
“Ported from musl”，另有 5 个头文件在去掉新增的版权行后与 musl 头文件
完全一致（`crypt.h`、`libintl.h`、`libgen.h`、`fmtmsg.h`、`tgmath.h`）。
当前 `LICENSE` 只有 BSD-3-Clause/Meng Zhuo，没有 musl 的 MIT 版权声明和
许可文本。

建议在发布前补充 `NOTICE` 或 `THIRD_PARTY_LICENSES`，保留 musl 的
copyright/permission notice，并在相关文件头保留来源说明。这不是对法律
结论的判定，但属于发布前应确认的合规项。

### 3.2 仍需代码修复的行为缺口

`docs/limitations.md` 已记录，但本轮没有改代码：

- stdio 线程安全、`random` 加锁；
- `wcwidth`/`wcswidth` Unicode 表；
- `getpagesize`/`sysconf(_SC_PAGE_SIZE)` 使用 `AT_PAGESZ`；
- `at_quick_exit` 回调表、`getpass` termios；
- 独立 loader 转发环境变量；
- `DT_NEEDED` 动态数组；
- `tsearch` 平衡树（性能而非正确性）。

### 3.3 需要定期复核的快照数据

`docs/performance.md`、`docs/workloads.md`、`docs/ltp.md` 和
`docs/workload-suites.md` 都是单机/单次快照。建议在每次 release 或
loader/libc 大改后重新运行并更新日期；`libc-test` 计数会随外部 checkout
变化，`docs/comparison.md` 已注明这一点。

## 4. 验证结果

```text
Markdown 相对链接/锚点检查    0 broken
cargo check -p keel-libc -p keel-rt   OK
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps   OK
keel-header-check             1407/1407 numeric macros match
```

完整 `cargo test --workspace` 未在本轮执行：审计期间同一工作树中还有
并发的安全修复改动（`libc/src/lib.rs` 的 printf 边界、`utils/` 等），
避免与并发工作互相干扰。
