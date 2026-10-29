# 三阶段路线：把 ROCKNIX 上的完整能力迁到 BaseOS

目标状态：插一张卡开机，BaseOS 直接进 PocketJS 卡片启动器，功能和现在 ROCKNIX 上一致，
并且 SSH / ADB / MTP / 浏览器远程调试这些开发基础设施都能用。

当前 runtime 的 AArch64 字符串表含有 PocketShell bridge 操作名、宿主命令和路径。
这是编译内容的证据，不是 BaseOS 上 bridge 已可用的验收证据。兼容层须按
现有 bridge 合约补齐宿主接口，不预设每项命令可直接工作。

---

## 阶段一：能开机进 PocketJS（交付待实机验证）

payload 由 `tools/make-payload.sh` 生成。当前 workspace 的静态检查和归档已完成；
BaseOS 实机启动、package admission、DRM 画面与按键还没有验证。

| 依赖 | 状态 |
|---|---|
| `System/launch_frontend.sh` | 已实现 DRM 探测、package `--check`、`/storage` best-effort bind、失败退避 |
| runtime + approved package | 从 inputs 组包；目标机 admission 未验证 |
| BaseOS `/storage` mount point | 需确认目标镜像提供目录及 mount 权限 |

验收：插卡进入批准版 HOME 卡片界面，日志记录 admission、bind mount、DRM card 和退出码。
设备步骤见 [DEVICE-CHECKLIST.md](DEVICE-CHECKLIST.md)。

---

## 阶段二：已有 PocketShell bridge + BaseOS 兼容层

runtime 字符串表已确认包含 `__pocketshell`、`__launcher`、
`POCKETSHELL_LIBRARY_ROOTS`、`POCKETSHELL_LIBRARY_DATA_ROOT`、
`pocketshell-games`、`pocketshell-play`、`systemd-run`、`systemctl`、
`pactl` 和 `amixer`。这证明代码被编进二进制，不代表对应宿主命令在
BaseOS 上已经可用。详细方案见 [MIGRATION.md](MIGRATION.md#stage-2-existing-pocketshell-bridge-compatibility-layer)。

当前交付：frontend best-effort `/storage` bind；`tools/build-compatibility-layer.sh`
可生成 games JSON scanner 和服务状态辅助脚本。scanner 已通过一个隔离 fixture；
两个 helper 尚未安装进 BaseOS PATH，也未在设备验证。

仍未实现/验证：BaseOS systemctl shim 集成、音量适配、游戏运行时交接、games
目录实机索引及 A 启动/退出回到 launcher。child runner 依赖 `pocket-session-run`
管理显示与服务交接；直接从仍持有 DRM 的前台启动第二个 runtime 可能冲突。
没有用通用后台 `systemd-run` 伪装成功。

验收标准：`games/gba/*.gba` 与根目录 `.pocket` 在 Library 分组正确；游戏可启动并
返回 launcher；状态栏/设置显示真实 BaseOS 状态。以上均未实机验证。

## 阶段三：开发基础设施（用户指定）

这一层是后续所有开发的底座，优先级和阶段二并行。

### 3.1 SSH

BaseOS 自带 dropbear，默认开启。需要做的：

- 验证默认可用（`ssh root@<ip>`），确认 root 登录方式与密钥位置
- 把 hostname / 认证方式写进 `baseos.conf` 与本文档
- bridge 的 `ssh.enabled` 状态映射到 `pidof dropbear`

### 3.2 ADB

BaseOS 自带 adbd，默认开启（USB）。需要做的：

- 验证 `adb devices` 能连上（之前 ROCKNIX 上 adbd 会掉，BaseOS 的 adbd 是原厂 harvest 的）
- bridge 的 `adb usb.enabled` 映射到真实 adbd 状态
- 保留 ROCKNIX 上那套 `pocketshell-dev` 的开关语义

### 3.3 MTP

ROCKNIX 有 MTP（`STATUS\tusb` 返回 `file_transfer`），BaseOS 只有 **USB 存储模式**
（`/usr/sbin/usb-storage-mode`，把卡当 U 盘挂给主机）。两个方案：

- **接 BaseOS 现成能力**：用 usb-storage 模式，UI 上的"USB 文件传输"改成"USB 存储"
- **补 MTP**：装 `mtp-responder`/`simple-mtpfs` 之类并写 gadget 配置，工作量大

先按第一个做（功能等价：能从电脑访问卡），需要真正 MTP 时再评估。

### 3.4 浏览器远程调试（掌机画面同步 + 按键注入）

这就是 `pocketshell-device`（`/usr/libexec/pocketshell-device --bind 127.0.0.1 --port 8080`）
提供的能力：`/device/frame.rgb` 抓 DRM 帧、`/device/key` 注入按键。
它是独立 Rust 服务，**需要在 HX90 上交叉编译 aarch64 版**，然后在 BaseOS 上以 init 脚本常驻：

- 编译：复用 ROCKNIX 的 Rust 工具链，产出静态或少依赖的二进制
- 启动：在 `launch_frontend.sh` 之前或之后拉起（注意它要和 runtime 争 DRM master，
  ROCKNIX 上的做法是只在调试时开）
- 安全：只 bind 127.0.0.1，外部通过 SSH 隧道访问（和现在 HX90 上的用法一致）

验收：`ssh -L 18080:127.0.0.1:8080 root@机子` 之后浏览器/curl 能抓帧、能注入按键。

### 3.5 其它已打包能力盘点

| 能力 | 来源 | BaseOS 处理 |
|---|---|---|
| `pocket` / `pocket-session-run` | ROCKNIX 脚本 | 需要 sh 重写（去掉 systemd 依赖） |
| `pocketjs-native-launch` | ROCKNIX 脚本 | 需要 sh 重写 |
| `pocketshell-legacy-settings` | ROCKNIX 脚本 | 按需移植 |
| `pocketjs-sleep` | 休眠服务 | 阶段三后期，和电源管理一起做 |
| samba / ftp / http / syncthing | bridge 里有开关 | BaseOS 不带，按需装 |
| `pocketshell-dev` | ROCKNIX 调试面板 | 依赖 8080，随 3.4 一起 |

---

## 风险与顺序

1. **阶段一必须先过**：镜像刷不出来，后面全是空谈。当前阻塞在 HX90 构建。
2. **阶段二 2.3（启动游戏）风险最高**：涉及 frontend 退出/恢复的时序，可能需要
   "启动器主动让出显示 → 子包跑 → 回来后重启启动器"的显式流程。
3. **阶段三 3.4 需要交叉编译**：HX90 有 ROCKNIX 的 Rust 环境，但要确认 `pocketshell-device`
   的依赖（DRM/evdev）在 BaseOS 内核头文件下能编过。
4. 全程遵循：**不在 Debian 上编译**，镜像和二进制一律在 HX90 产出。

---

## 阶段四：发版与自愈（让它能持续交付）

前三阶段让它跑起来，这一阶段让它能持续活下去。动机很直接：现在的交付链路是
"手动编镜像 → 手动写卡 → 手动插卡验证"，而多个 agent 同时在改同一批代码，
今晚的事故（runtime 被换、profile 不匹配、开不进去）就是这条链路的必然产物。

### 4.1 一条命令出三样东西

HX90 上执行一次，产出镜像、payload、版本清单，三者出自同一次构建：

```
dist/
  baseos-rg34xx-<ver>.img.zip
  sdcard-payload.tar.gz
  build-manifest.json      # launcher 源码 commit、runtime sha256、package sha256、构建时间
```

不允许"镜像在这台机器编、payload 在那台机器打包"这种跨机器拼装。

### 4.2 卡上可追溯

卡根 `System/pocketjs/version` 已经记录 runtime 和 package 的 sha256；阶段四补上
源码 commit 和构建 ID。任何一张卡都能回答"这是哪次构建、哪个 commit、哪个包"。

今晚为了确认设备跑的是 4,084,728 还是 3,910,008，只能靠人工比对哈希；这个判断要自动化。

### 4.3 开机自检

首次启动跑一遍，结果写卡根 `selfcheck.json`：

| 检查 | 判据 |
|---|---|
| runtime 可执行 | `[ -x ]` + `--version` |
| 包被接纳 | `--check` 输出 `package admitted` |
| DRM 节点 | `/dev/dri/cardN` 存在 |
| `/storage` 映射 | `mountpoint -q /storage` |
| Library 索引 | `library.json` 存在且条目数 > 0 |
| SSH / ADB | `pidof dropbear` / `pidof adbd` |
| 前端服务 | baseos.conf 可读、hostname、IP |

插卡后看一个文件就知道走到哪一步断的，不用再连环 ssh 猜。

### 4.4 失败回滚

保留上一版 payload（`.backup/System-<ts>.tar.gz` 已有雏形）。阶段四补：
连续启动失败 N 次后自动回退上一版，并把失败日志一起留在卡上。镜像走 BaseOS 原生
A/B slot 回滚，payload 走卡内备份。

### 4.5 双通道更新

- **镜像**：BaseOS 原生 `.bosupd`，复制到卡根重启即生效
- **payload**：启动器 UI 和资源单独热更新，换一版界面不需要刷镜像

这条直接决定开发速度：现在改一行 UI 都要重编 4 GB 镜像，不可接受。

### 4.6 性能预算

- 开机到启动器首帧：BaseOS 原生约 3 s，加上 PocketJS 后设上限（暂定 6 s）
- 帧率基线：PocketJS 逻辑 360×240 @2x，目标稳定 60 FPS
- 任何改动超标即视为回归，写进构建产物

### 4.7 可持续性

- `PJOS-LINUX` 的改动（`overlay/storage` 等）集中记录在 `docs/BASEOS-PATCHES.md`，
  便于跟随上游 rebase
- 构建脚本可重入：同样的输入产出同样的产物（除时间戳），能定位"昨天能编今天不能"
