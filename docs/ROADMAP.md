# 三阶段路线：把 ROCKNIX 上的完整能力迁到 BaseOS

目标状态：插一张卡开机，BaseOS 直接进 PocketJS 卡片启动器，功能和现在 ROCKNIX 上一致，
并且 SSH / ADB / MTP / 浏览器远程调试这些开发基础设施都能用。

关键前提（已用 `strings` 从 `inputs/pocket-linux-kms` 验证，不是推测）：
**我们手上的 runtime 二进制本身已经编入了 PocketShell bridge**，包括

```
__pocketshell  __launcher
POCKETSHELL_LIBRARY_ROOTS   POCKETSHELL_LIBRARY_DATA_ROOT
/storage/pocketshell/library/library.json
/usr/bin/pocketshell-games  /usr/bin/pocketshell-play
systemd-run --unit=pocketshell-game ... / systemctl / pactl / amixer
adbd.enabled adb usb.enabled ssh.enabled avahi ftp http rsyncd samba syncthing telnet pocketshell-dev.enabled
```

所以**不需要重新设计 ABI、不需要重编 runtime 就能恢复大部分功能**。
二期的工作量集中在"把 bridge 依赖的宿主接口补齐"，而不是改 Rust。

---

## 阶段一：能开机进 PocketJS（进行中）

交付物已经在 `System/` `tools/` `docs/` 里，payload 由 `tools/make-payload.sh` 生成。

| 依赖 | 状态 |
|---|---|
| BaseOS 镜像 rg34xx | HX90 构建中（`build-tools → rootfs → qemu smoke → image`） |
| `System/launch_frontend.sh` | 完成（含 DRM 探测、admission 检查、失败退避、`/storage` bind mount） |
| runtime + 包 | `inputs/` 里现成，admission 已验证 `rocknix-h700-kms` |
| 镜像内 `/storage` 挂载点 | 已加 `PJOS-LINUX/overlay/storage/.keep`（根目录只读，必须预留） |

验收标准：插卡开机 → 出现卡片启动器 HOME 页（Terminal / Files / XMPlayer + 顶部状态栏），
`/tmp/pocketjs-frontend.log` 里能看到 `bound /mnt/sdcard -> /storage` 和 runtime 启动记录。

---

## 阶段二：已有 PocketShell bridge + BaseOS 兼容层

runtime 字符串表已确认包含 `__pocketshell`、`__launcher`、
`POCKETSHELL_LIBRARY_ROOTS`、`POCKETSHELL_LIBRARY_DATA_ROOT`、
`pocketshell-games`、`pocketshell-play`、`systemd-run`、`systemctl`、
`pactl` 和 `amixer`。这证明代码被编进二进制，不代表对应宿主命令在
BaseOS 上已经可用。详细方案见 [MIGRATION.md](MIGRATION.md#stage-2-existing-pocketshell-bridge-compatibility-layer)。

实施项：

1. 验证并使用 frontend 的 `/mnt/sdcard` → `/storage` bind mount。
2. 追踪 `POCKETSHELL_LIBRARY_ROOTS` 的真实消费路径；确认能跳过
   `pocketshell-games` 后才采用直接扫描，否则写 BusyBox shell index shim。
3. 为游戏退出回到 frontend 设计窄范围 `systemd-run` / `pocketshell-play`
   兼容路径；必须验证 DRM/input 所有权及 init respawn 时序。
4. 按 BaseOS 实际进程和音频接口扩展 systemctl、pactl/amixer 适配，逐项
   确认设置页显示真实状态。

当前状态：只完成启动脚本中的 best-effort `/storage` bind mount 与规划；
Library index、systemd-run shim、游戏启动、服务及音频适配都未实现或验证。
不能把 strings 证据当作功能验收。

---

### 2.3 启动游戏（`systemd-run` 缺失）

bridge 用 `systemd-run --unit=pocketshell-game --collect --no-block <play ...>` 起游戏。
BaseOS 没有 systemd-run，需要 shim：

```sh
# /usr/bin/systemd-run  (BaseOS shim)
# 忽略 --unit/--collect/--no-block，把剩余命令放后台跑，立刻返回成功
```

同时 `pocketshell-play` 原实现依赖 `pocket-session-run` + systemd 的显示锁，
BaseOS 上要改成：停掉 frontend（或者直接让 frontend 退出）→ 跑子包 → 退出后由 init
respawn 回到启动器。**这是二期最容易出问题的一环**，要单独验证"启动游戏 → 退游戏 → 回启动器"。

### 2.4 状态与控制

| bridge 调用 | BaseOS 现状 | 处理 |
|---|---|---|
| `systemctl is-active adbd/ssh/avahi/...` | 有 shim，但只认真实 unit，其余返回 3 | 扩展 shim：按 `pidof` 判断 adbd/dropbear/avahi-daemon |
| `systemctl try-restart pocketjs-sleep.service` | shim 返回 0（noop） | 可接受；休眠另做 |
| `pactl get-sink-volume` / `amixer` | 需确认是否安装 | 优先 `amixer sget`，pactl 用 shim 包装 |
| `/sys/class/backlight`、`/sys/class/power_supply` | 内核自带 | 直接可用，无需处理 |
| `/storage/.config/system/configs/system.cfg` | 无 | 首次启动生成默认值即可 |

验收：状态栏电池/亮度/WiFi 显示真实值；设置页里 SSH/ADB 开关能反映真实进程状态。

---

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
