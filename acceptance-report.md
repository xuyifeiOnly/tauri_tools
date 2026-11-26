# Mac Tunnel 开发预览验收记录

日期：2026-11-24。状态：已生成开发包，尚未完成正式验收。

## 已验证

### macOS Dock 恢复窗口（2026-11-26，源码）

- 补充macOS `RunEvent::Reopen`调用show_main_window，对Dock重新打开恢复隐藏/最小化窗口并聚焦；不新建窗口或重启共享。此前仅处理托盘打开与单实例第二进程，缺少此事件。
- 编译通过；并发Rust全套25通过/1失败，既有stop_releases_local_listener_before_returning出现AddrInUse；串行重跑26项全通过，未修改该测试掩盖问题。cargo fmt及diff检查通过。未安装或原生Dock点击验收，不宣称已复现/彻底解决用户所有打不开场景。

### 应用单实例与已有窗口唤起（2026-11-26，源码）

- 新增官方tauri-plugin-single-instance并作为Builder第一个插件注册，重复启动时唤起已有窗口，不再初始化第二套配置、托盘、运行时；show_main_window统一show/unminimize/set_focus并供托盘复用。
- `cargo test`26项、`bun test`18项63条断言、`bun run build`类型检查/构建通过，cargo fmt完成；Cargo.lock保留原有版本修改并新增插件依赖。
- 未执行原生重复启动/并发启动/隐藏或最小化唤起、Windows安装与退出重启验收。未打包安装或操作当前实例；旧版本已启动的多实例不会自动回收，首次升级须正常退出旧实例。WebView与SSH合法子进程不属于重复客户端。

- 2026-11-26：新生成密钥注释改为 `suixiang-xyf-key`；`cargo test --manifest-path src-tauri/Cargo.toml keys::tests::generated_key_can_be_used_for_deployment` 通过，断言生成公钥的注释并验证部署兼容。未修改已有密钥，未打包安装。

### 独立 TypeScript 版本同步脚本（2026-11-26）

- 新增 `scripts/bump-version.ts` / `bun run version:bump`：默认patch+1，支持显式稳定版、预览、跨工作目录路径解析。先校验三处再写入，保持其余格式与依赖；写入失败尝试恢复本次已写文件。不打包安装或发布，Cargo.lock留待Cargo更新。
- `bun test`18项63条断言、`bun run build`类型检查/构建、`git diff --check`通过；真实仓库仅执行dry-run，输出0.1.2→0.1.3，未修改实际版本。未执行打包、安装或远端发布。

### Windows 安装包 ZIP 分发（2026-11-26，源码）

- Windows NSIS 构建后使用 PowerShell Compress-Archive 生成 `suixiang_<版本>_windows_x64.zip`，内含唯一原始 exe；自动解压回读验证文件数量/名称与 SHA-256，再上传 artifact。Release 只上传 ZIP，检查双平台 ZIP 均存在；macOS 打包不变。
- 本地验证：工作流 YAML 解析、ZIP 上传配置/校验步骤静态检查、`git diff --check` 通过；`bun test`15项55条断言、`bun run build`类型检查及构建通过。未在本机执行Windows PowerShell归档步骤。
- 不自动删除已有 Release 的历史 exe，不改版本，不推送或实际发布。ZIP 是安装程序包装而非便携版，也不能据此宣称已修复此前“损坏”问题。Windows归档和安装仍需GitHub runner/实机验收。

### Windows SSH 控制台弹出与退出诊断（2026-11-26，源码）

- 确认原 SSH 启动未设置 Windows 无控制台标志；统一后台命令构造器设置 CREATE_NO_WINDOW、stdin null，覆盖异步 SSH/指纹扫描、同步密钥及 Windows 网络/打开地址工具。保留 stderr 捕获、严格主机校验、心跳和退避，不改真实服务器或现有隧道。
- 退出日志增加进程退出码；远端端口绑定失败提示其他设备/旧隧道可能占用，不自动杀其他设备。截图不能确认实际断线原因，仍需 Windows 日志。
- macOS `cargo test` 26项通过（新增退出255/远端转发失败日志回归）；`bun test`15项通过；`bun run build`类型检查与构建通过；cargo fmt完成。
- Windows 专属两项测试覆盖 CREATE_NO_WINDOW 下无控制台、stderr及异步退出码，已接入 GitHub Windows 构建 job，但本轮未执行Windows测试/打包/安装或真实SSH连接，不能宣称Windows稳定性已验收。无UI改动，未进行页面回归；未推送、发布或安装。

### 手动工作流发布 Release（2026-11-25，源码）

- 保留当前双平台构建矩阵，新增 prepare 校验三处源码版本及标签；手动运行使用 `vX.Y.Z`，两个平台成功后发布 Windows exe 和 macOS zip，保留 artifact。
- 发布 job 独占 `contents: write`；新建标签指定本轮 commit；同版本重跑更新附件，不移动已有标签。未配置签名/公证，不增加平台或架构。
- 本地验证：`bun test` 15 项、55 条断言通过（含发布元数据 4 项）；`bun run build` 类型检查及构建通过；元数据命令输出版本 0.1.2/标签 v0.1.2；Ruby YAML 解析及 release 依赖/无手动触发限制检查、`git diff --check` 通过。本轮不推送、不触发远端工作流，不代表 GitHub 双平台构建或 Release 发布已实测。

### Windows 跨平台构建支持（2026-11-25，源码）

- 新增 `src-tauri/src/platform.rs` 集中封装平台差异：SSH/ssh-keygen/ssh-keyscan 路径（Windows 优先 `System32\OpenSSH`，回退 PATH）、`open_url`（macOS `open` / Linux `xdg-open` / Windows `cmd /C start`）、`set_private_file`/`set_private_dir`（unix 设 600/700，Windows 忽略）、`lan_addresses`（macOS `ifconfig` / Linux `ip` / Windows `ipconfig`）、`ssh_config_null`（`/dev/null` vs `NUL`）。
- `lib.rs`、`config.rs`、`keys.rs`、`runtime.rs`、`browser.rs`、`orphan_ssh.rs` 改为调用平台层；macOS 专属的孤立 SSH 检查用 `cfg!(target_os = "macos")` 限定，其他平台跳过，避免 Windows 连接被“不支持”错误阻断。unix 专属测试（`/bin/kill`、`/bin/sleep`、`PermissionsExt`、符号链接）按 `cfg(unix)` 隔离。
- `tauri.conf.json` 的 `bundle.targets` 改为 `all`，图标增加 `icons/icon.ico`（由 `tauri icon` 生成后仅保留 ico）。新增 `.github/workflows/build.yml`：`windows-latest` 构建 NSIS 安装包、`macos-latest` 构建 `.app` 并打包为 zip，统一发布至 Release（详见上文“手动工作流发布 Release”）。
- 验证：`cargo check`/`cargo test`（macOS）25 项通过；`bun run build` 类型检查与构建通过；`bun test` 11 项通过；工作流 YAML 通过 `YAML.load_file` 校验。
- 未覆盖：Windows 原生编译/运行未在本机验证（macOS 无法交叉编译 `ring`），需 GitHub Actions 或 Windows 机器实测；Windows 下 OpenSSH 路径、托盘、自启动、局域网访问均未实测。

### 打包默认 release 与自动递增版本（2026-11-25，源码）

- 新增 `scripts/package.sh` 作为统一打包入口：默认 release、默认 patch 版本 +1；`--debug` 才生成 debug 包；保留 `--version X.Y.Z`、`--bump major|minor|patch`、`--keep-version`、`--dry-run`。版本同步 `package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`，只替换顶层 version 唯一匹配，不误改依赖版本。
- `scripts/app_install.sh` 默认改为 release 并新增 `--debug`，版本同步与构建委托给 `scripts/package.sh`，避免一次操作重复递增版本；保留 stop/check、暂存、原子替换、bundle cleanup、`--no-install` 行为。`package.json` 新增 `package` 脚本（`bun run package`）。
- 验证：`bash -n scripts/package.sh`、`bash -n scripts/app_install.sh` 通过；`--dry-run` 默认显示 release 且版本 0.1.2→0.1.3 不写文件；`--debug`/`--keep-version`/`--bump minor`(0.2.0)/`--version 0.3.0`/非法 `--bump` 语义正确；临时副本验证真实写入三处版本一致且依赖版本不变。`python3 scripts/test_app_install.py` 6 项、`python3 scripts/test_service.py` 3 项通过。
- 未覆盖：真实 release 打包、安装覆盖、原生启动；未修改服务器或现有隧道。

### 共享工作台优化（2026-11-25，未安装）

- `bun run build` 和 `bun test` 通过（11 项）；Rust 23 项通过。新增测试确认相同配置不停止隔离的本地服务、不写盘，实际修改会停止服务并持久化；链接测试覆盖本机回环、目录未保存和公网未验证。
- Ego Lite，`http://127.0.0.1:1420/`，TaskSpace 12/p1，无原生后端。亮色 1000×700：概览主操作与三阶段状态在首屏；目录新增后路径在首位、保存栏固定且无横向溢出。暗色 1200×800：连接配置、高级设置展开与固定保存栏正常，无横向溢出。实际点击验证无修改禁用保存、新目录地址提示先保存、预览保存报错且保留草稿。
- 截图位于 `/private/var/folders/gx/qmlmrzyn4kb33r1zm1bzkwbw0000gn/T/opencode/`：`suixiang-overview-1000.png`、`suixiang-mapping-1000.png`、`suixiang-settings-dark.png`。未执行真实保存密码、外部浏览器打开、原生界面、现有隧道或公网；没有覆盖安装。

### 连接表单溢出与输入焦点（2026-11-25，未安装）

- 表单标签显式minmax(0,1fr)单列、select限宽，避免长密钥指纹撑宽隐式网格；滚动区域仅纵向auto、取消stable空槽，内边距6px。输入/select统一2px主题色outline及1px偏移，不再被边界裁切。连接表单缩减纵向间距、移除空状态段落、分隔线使用主题token。
- Ego Lite TaskSpace10/p1，真实无后端预览`http://127.0.0.1:1420/`，实际点击连接配置、输入焦点、下拉、切换暗色、滚动：注入仅DOM测试长选项后仍无横向溢出。1200×707区域clientWidth/scrollWidth均861；焦点框完整，左侧6px余量。1200×1000区域clientHeight/scrollHeight均629，无横纵滚动。1000×700区域宽度均688、根宽1000；滚动后后台运行可见且window.scrollY为0。较矮窗口真实内容溢出时保留纵向滚动，不能称所有尺寸都无需滚动。
- 截图已查看：`/private/var/folders/gx/qmlmrzyn4kb33r1zm1bzkwbw0000gn/T/opencode/connection-focus-fixed.png`、同目录`connection-no-overflow.png`、`connection-focus-dark.png`。TaskSpace已finish，测试DOM数据未持久化。
- `bun test`10项42条断言、`bun run build`类型检查及构建通过。未验证原生WebView/macOS常驻滚动条设置、真实密钥选择/保存、自启动；未安装、未停止共享或修改服务器。

### 目录映射电脑IP与LAN设置样式（2026-11-25，未安装）

- 本地打开与复制统一使用电脑私网IPv4，每次查询系统接口；单IP直接执行、多IP按需选择。复用fileLink处理目录编码，不回退127.0.0.1。检查目录启用/LAN授权、保存配置中的端口/目录路径及路由一致、本地运行状态；公网逻辑不变。
- LAN选项改为紧凑左对齐，复选框16px、与文字相隔8px，12px辅助说明紧跟并允许换行。
- `bun test`10项42条断言通过，包含目录根链接和特殊字符路由/自定义端口；`bun run build`类型检查及构建通过。
- Ego Lite TaskSpace9/p1、1200×707，临时`http://127.0.0.1:1420/lan-mapping-fixture.html`明确模拟IPC及剪贴板：点击本地打开/复制得到相同`http://192.168.1.20:20005/test_page/`；多IP选择后复制`http://10.0.0.20:20005/test_page/`；未授权、未保存、服务未运行、无IP错误反馈均通过。实测标签间距8px、无横向溢出，亮暗布局截图已检查。测试页已删除、TaskSpace已finish。
- 截图：`/private/var/folders/gx/qmlmrzyn4kb33r1zm1bzkwbw0000gn/T/opencode/mapping-lan-light.png`、同目录`mapping-lan-dark.png`。暗色宿主按钮浅底现象仍在，不标记原生暗色按钮验收通过。
- 未覆盖：真实IPC/系统剪贴板/外部浏览器、多网卡实际可达性、防火墙与跨设备访问。未打包安装、未重启当前服务、未修改服务器。

### 连接前孤立SSH检查（2026-11-25，未安装）

- 新orphan_ssh模块使用sysctl真实argv完整匹配应用SSH命令，限定同用户/PPID1，发送TERM前复核UID/PPID/lstart及argv，5秒等待；其他客户端管理的连接报冲突，核验失败不建立新连接。非模糊ps文本匹配，不批量杀SSH，不接管不同配置/远端未知连接。Runtime先回收当前任务，再执行一次检查。
- Rust22项通过：新增完整argv（含空格路径）、所有参数改变拒绝、其他用户/活跃父进程保护、当前进程sysctl真实argv读取。前端10项与类型检查/构建通过。
- 本轮未停止真实连接、未调用真实孤立SSH清理、未模拟PID复用/TERM失败及远端释放，也未覆盖安装；sysctl/ps复核降低误杀风险，但核验与信号之间仍有极短竞态窗口，不宣称原子身份绑定。与本地入口修复均待安装后验收。

### 升级后重连等待与概览入口（2026-11-25）

- 现场确认本地20005入口HTTP200。升级前旧SSH PID64677 PPID1仍占18790，当前随享PID92492；仅对已核对完整主机/端口/密钥路径/转发参数的64677发送TERM。当前客户端自动创建95734（PPID92492），公网健康HTTP200且正文匹配本地实例。未修改Nginx或其他SSH。
- 概览入口原为普通target=_blank，未调用系统open_url；源码改用现有openAddress并展示失败Toast。前端10项测试、类型检查及构建通过；未进行本轮Ego点击/原生外部浏览器验收，不宣称UI端到端通过。
- 链接修复尚未安装，以免未经确认再次中断已恢复的公网；本次事实不证明新版退出遗留，发现的PID64677是升级前旧进程。新版正常退出回收仍待真实验证。

### 随享改名与重复安装（2026-11-25）

- productName、窗口标题、侧栏、菜单栏、共享页与安装包改为随享；HTTP Basic realm使用ASCII Suixiang避免非法头。保留原identifier、二进制名与数据目录，不迁移或读取输出密钥/密码。
- 新增app_bundles.py，限定安装目录、~/Applications、/Applications和项目debug/release打包目录按标识核验；错目标和危险符号链接拒绝处理。安装后真正移除重复包并只注册目标，非仅注销，清理失败报错。临时替换使用.bundle而非.app。
- 首次0.1.2安装已复制新版但清理遇到lsregister -10814（未注册的新构建包）而中止；已补“注册后再注销”兼容与单元测试，不吞掉其他错误。随后--keep-version安装成功，再次完整构建/覆盖安装成功。实际旧Mac Tunnel安装包、旧debug/release以及新debug构建副本均清除。
- 最终限定目录扫描仅`/Users/xuyifei_coco/Applications/随享.app`，LaunchServices dump仅一条identifier cn.rutang.mac-tunnel，名称随享、版本0.1.2。客户端保持退出，不自动启动共享，不更改服务器/手工隧道。
- 安装安全回归6项、服务脚本3项、bun10项、类型检查/构建通过；Rust19项通过，cargo fmt完成。未进行Ego页面或启动台视觉验收、原生启动/共享回归、登录启动迁移；系统注册唯一不等同图标刷新实测。直接Tauri构建/--no-install及其他位置手工复制不属安装后去重保证。

### 手动公网验证与10分钟自动检查（2026-11-25）

- 右上角新增“验证公网”，等待时显示“正在验证…”并禁用；结果用Toast反馈并刷新状态。本地未启动或SSH进程未运行时禁用。后端同一SSH任务串行探测，每次进程启动约3秒首检，后续完成起600秒自动检查；手动检查重置期限，不改变SSH心跳或前端status刷新。
- Rust19项测试通过，新增手动检查无需等待10分钟、自动检查等待截止时间/未连接拒绝请求。bun test10项、类型检查和构建通过。
- Ego Lite TaskSpace8/p1，`http://127.0.0.1:1420/public-check-fixture.html`模拟IPC：实际点击按钮、观察验证中禁用、完成成功/失败提示和公网状态刷新通过；未连接禁用及真实IPC未在本轮浏览器验证。截图：`/private/var/folders/gx/qmlmrzyn4kb33r1zm1bzkwbw0000gn/T/opencode/public-check-pending.png`、`public-check-failed.png`。临时页已移除，不修改真实服务。
- 未安装、未重启客户端、未更改服务器；未实际等待10分钟或进行真实公网手动验收。两次检查间显示上次结果，不能宣称持续可达。

### 主机信任按钮反馈（2026-11-25）

- DeploymentPanel仅trust_host成功后显示绿色勾选“已确认信任”、禁用重复点击；读取及主机/端口变化清除确认，异步revision避免旧响应变绿。失败允许重试。未增加持久化可信状态查询，离开页面/重启后不恢复绿色。
- Ego Lite TaskSpace 7 / p1，临时`http://127.0.0.1:1420/trust-fixture.html`模拟IPC：实际点击读取/确认后文字、禁用和绿色通过；重新读取清除、模拟失败不误标绿、修改主机清空通过。亮暗截图保存在`/private/var/folders/gx/qmlmrzyn4kb33r1zm1bzkwbw0000gn/T/opencode/trust-confirmed-light.png`及`trust-confirmed-dark.png`。暗色宿主禁用按钮底色偏浅仍存在，未宣称原生视觉验收。测试页已移除，未修改真实known_hosts。
- bun test 10项通过、前端类型检查/构建通过；未覆盖原生IPC真实保存、跨页面恢复和安装，未停止当前隧道。

### SSH生命周期清理（2026-11-25，源码修复）

- `Runtime`正常停止不再只abort：oneshot通知持有Child的任务，显式kill/wait后join；重复连接先等待旧进程回收，退避和健康请求可中断，读取stderr任务停止并等待。停止本地服务取消任务后等待完成。
- Tauri托盘退出、Cmd+Q/系统正常退出统一ExitRequested入口；清理成功后允许第二次exit，退出期间禁止启动/连接/配置修改。窗口关闭保持隐藏。Drop发送停止信号兜底；SIGKILL/崩溃不作清理承诺，不扫描或泛杀历史SSH。
- 新测试先确认旧接口不支持await/测试驱动而编译失败，再实现。`cargo check`通过；最终Rust17项通过（新增6项：真实假进程PID消失、重复连接先回收、退避取消、挂起健康请求可停止、Drop通知、监听端口释放）；runtime测试额外连续5轮通过。一次并发全套中端口立即重绑测试报AddrInUse，单项、随后全套及5轮均未复现，原因未定，保留记录不宣称压力稳定性。
- `bun test`10项通过；`bun run build`类型检查及构建通过。没有UI变更，未进行浏览器回归。
- 未覆盖安装、未停止当前公网/手工隧道、未修改服务器。原生托盘/Cmd+Q/安装脚本请求退出及真实远端端口释放仍需安装新版后验收；当前运行旧安装不自动获得修复。强制退出/崩溃、网络断开远端释放延迟不保证。

### 真实公网连接排障（2026-11-25）

- 用户授权排查重连等待、验证公网及必要时修改Nginx。新服务器`82.156.234.208:22`公钥认证成功；Mac孤立旧SSH PID 61817（PPID 1）占用远端18790，当前应用重复绑定失败。核对完整目标/转发参数后仅向该孤立进程发送SIGTERM；原有旧服务器18788手工隧道PID 1674保留。
- OpenResty容器`1Panel-openresty-P5o4`使用host网络。`/opt/1panel/apps/openresty/openresty/conf/default/00.default.conf`已含`/my_mac/`到`127.0.0.1:18790`的保留路径代理，但运行配置未生效，公网404。未改文件内容；先备份为同目录`00.default.conf.bak-20261125-134541`，`nginx -t`通过后`nginx -s reload`平滑重载。重连间隙短暂502，应用自动重连后恢复。
- 当前SSH PID 64677的父进程为已安装客户端PID 62541；远端18790正常监听。公网`http://82.156.234.208/my_mac/_health`返回200，实例标识与本地完全一致。公网PDF Range读取返回206及正确Content-Range、PDF文件头。
- Ego Lite TaskSpace 6 / p1：真实打开`http://82.156.234.208/my_mac/`并点击“测试目录”，目录列表显示PDF及子目录，通过。截图：`/private/var/folders/gx/qmlmrzyn4kb33r1zm1bzkwbw0000gn/T/opencode/public-tunnel-directory.png`。非模拟数据。未对公网文件执行写入或删除。
- 未覆盖：客户端原生状态文字确认、长期稳定、断网/睡眠恢复、HTTPS、其他目录/全部页面资源。当前HTTP传输未加密；不在此协议下建议发送Basic密码。旧SSH退出遗留为已复现生命周期缺陷，尚未修改代码根治；此次只清理确认的旧进程。

### SSH主机文件路径与状态配色（2026-11-25）

- 定位到`UserKnownHostsFile`未经OpenSSH配置语法引用，`Application Support`被拆为多个文件路径。修复为双引号引用并转义反斜杠/引号，拒绝换行、回车、NUL和无效UTF-8；保留StrictHostKeyChecking=yes，不替换已信任指纹。
- 系统SSH对比握手（禁用登录认证、无-R转发）：原参数读不到指纹文件，引用路径后能匹配已信任服务器主机公钥。此验证不证明账号公钥登录、隧道或公网成功；未修改服务器/Nginx。
- `bun test`10项通过，`bun run build`类型检查和构建通过；`cargo test --manifest-path src-tauri/Cargo.toml`11项通过，新增路径引用/转义/换行拒绝测试。
- Ego Lite TaskSpace 5 / p1，`http://127.0.0.1:1420/status-fixture.html`临时模拟IPC页：通过异常红色、本地运行绿色、暗色SSH进程与公网可访问绿色的用户可见检查；模拟页已删除。截图在`/private/var/folders/gx/qmlmrzyn4kb33r1zm1bzkwbw0000gn/T/opencode/`：`status-error-light.png`、`status-error-dark.png`、`status-running-dark.png`。仅配色验收，不代表真实服务已连接。
- 未打包安装，未验证修复后原生SSH连接、公钥登录及真实公网访问；旧安装仍可能报同一错误。

### 固定框架与分区滚动（2026-11-25）

- 新增`src/desktop-layout.css`：窗口根节点不滚动，左导航/右顶栏固定；连接表单、目录和密钥列表独立滚动并固定主要操作，脚本限高240px、日志独立滚动。目录弹窗改为flex高度约束，仅文件列表滚动。
- `bun test`9项、`bun run build`类型检查和构建通过。
- Ego Lite：`http://127.0.0.1:1420/`，TaskSpace 4 / p1，真实预览1200×707。通过：滚轮滚动连接表单202px时window.scrollY、导航/顶栏top仍为0，保存footer位置不变；Home/End聚焦表单后可看到后台运行末尾；保存仍报明确无原生后端错误；新增6个目录后列表滚动1176px而导航/顶栏不动、保存仍可见，亮暗切换及截图检查。
- 临时`/scroll-fixture.html`明确标注模拟IPC/密钥/日志/部署，测试后已删除。通过：8个长指纹/路径密钥列表不撑宽；长脚本内部滚动676px而外内容scrollTop仍0；300行日志滚动980px时框架不动。1000×700根scrollWidth/scrollHeight正好1000/700，连接表单滚动159px时保存底部651px仍可见；1200×800暗色连接截图通过布局检查。未执行真实生成密钥、部署或读取私钥。
- 截图：`/private/var/folders/gx/qmlmrzyn4kb33r1zm1bzkwbw0000gn/T/opencode/scroll-connection-1000.png`、`scroll-connection-dark-1200.png`、`scroll-mappings-dark.png`（后两者与前者同目录）。宿主浏览器暗色按钮浅填色已知现象保留，不将原生主题呈现标为通过。
- 未覆盖：原生WebView实际窗口缩放/触控板、文件弹窗高度边界专项、系统总是显示滚动条设置、低于最小1000×700窗口。没有打包安装、停止服务或操作公网。

### 文件浏览紧凑改版（2026-11-25）

- 彩色Lucide文件类型图标、36px行、28px复制按钮，移除常驻地址栏；每文件独立LAN/公网复制，多网卡点击后再选。保留目录授权、已保存校验和公网可达性要求。
- `bun test`：9项通过（新增3项含分类、双通道独立性、路径编码及禁用/未启动/未授权/无IP/无源站/公网不可达校验）；`bun run build`类型检查与构建通过。
- Ego Lite：TaskSpace 3 / p1，1200×707，`http://127.0.0.1:1420/`真实预览新增目录并打开浏览弹窗，原生后端缺失错误明确展示，通过。未操作磁盘或真实网络。
- 同一TaskSpace中临时`/browser-fixture.html`明确标注模拟数据及模拟IPC/剪贴板，已移除测试页。通过：目录展开/收起、键盘选择多网卡IP、单网卡直接复制、公网独立复制中文/#编码、不可达公网报错后LAN仍可复制、关闭/重开及Escape。模拟写入分别得到`http://10.0.0.20:20005/docs/1101.pdf`及`https://example.com/my_mac/docs/demo_html/%E4%B8%AD%E6%96%87%20%23.html`。
- 亮暗截图检查、实测36px/28px、列表无横向溢出通过，暗色图标使用独立浅色token。截图：`/private/var/folders/gx/qmlmrzyn4kb33r1zm1bzkwbw0000gn/T/opencode/file-browser-light.png`、`/private/var/folders/gx/qmlmrzyn4kb33r1zm1bzkwbw0000gn/T/opencode/file-browser-dark.png`。宿主暗色关闭按钮填色偏浅现象仍存在，不宣称原生样式已验收。
- 未覆盖原生IPC/系统剪贴板、多网卡实际可达、真实LAN/公网访问、深层目录压力/全类型文件/空与加载专项。未打包安装或修改服务器。

### 启动体验优化（2026-11-25）

- 未保存配置不再禁用本地启动，改为“保存并启动本地服务”；先保存再启动，分阶段文案及防重复提交；运行中禁用重复启动，dirty时保留带停止提示的保存入口。公网连接展示缺失条件，首次指引分本地与可选公网。
- `bun test tests/service-controls.test.ts`：6项通过，26条断言，覆盖顺序、保存失败不启动、启动失败保留已保存状态、阶段文案、重复提交、无服务器/密钥本地启动及连接前提。`bun run build`类型检查与构建通过。
- Ego Lite：`http://127.0.0.1:1420/`，TaskSpace 2 / p1，1200×707，浏览器预览。通过真实点击直接启动显示原生后端缺失错误；新增目录、确认未保存切换、点击“保存并启动”失败后按钮仍可用且服务未启动；公网按钮禁用并列明保存/启动/服务器/密钥条件；本地/公网指引文案可见；亮暗切换、无横向溢出。
- 截图：`/private/var/folders/gx/qmlmrzyn4kb33r1zm1bzkwbw0000gn/T/opencode/mac-tunnel-start-light.png`、`/private/var/folders/gx/qmlmrzyn4kb33r1zm1bzkwbw0000gn/T/opencode/mac-tunnel-start-dark.png`。暗色截图中宿主浏览器部分按钮填色偏浅，为现有宿主呈现问题，本次未把原生暗色样式标为通过。
- 未覆盖：原生保存/启动成功链路、运行中dirty保存按钮真实服务回收、公网连接。顺序与成功状态由单元测试覆盖，不替代原生验收。未覆盖安装、未连接服务器、未停止既有服务。

- 图标增量（2026-11-24）：已将可编辑 SVG 生成 macOS PNG/ICNS，前端侧栏复用同一设计；`bun run build` 和 `bunx tauri build --debug` 通过，debug `.app` 的 Resources 内含新 `icon.icns`。Codex 内置浏览器在 `http://127.0.0.1:1420/` 检查概览页，侧栏新图标可见且与文字对齐。未检查 macOS 菜单栏实际小尺寸显示及 Dock 中的原生呈现。
- `bun run build`：通过 vue-tsc 类型检查及 Vite 生产构建。
- `cargo test --manifest-path src-tauri/Cargo.toml`：6项通过，包括配置注入校验、600权限、脚本引用、路径越界/符号链接、真实回环HTTP认证及Range、本地端口占用不虚报运行。
- HTTP测试：本地 `/docs/` 和公网前缀 `/my_mac/docs/` 均实施认证，未认证子资源返回401；认证后HTML200；Range206；POST405；健康实例标识匹配。
- `bun run tauri build --debug`：通过，生成 `.app`，约41.36MiB；仅本机架构，无Developer ID签名/公证。

## Ego Lite 浏览器检查

环境：`http://127.0.0.1:1420/`，TaskSpace 8 / p1，1200×706，明确显示无原生后端预览。

- 通过：目录新增、名称/路径输入、移除映射后显示空状态。
- 通过：预览保存返回明确错误，不伪装已成功持久化。
- 通过：未保存离开确认后切换连接配置，表单字段可见。
- 通过：亮暗切换；截图检查连接表单及概览；1200px下无横向溢出。修正暗色按钮受系统默认背景影响的问题。
- 截图：`/private/var/folders/gx/qmlmrzyn4kb33r1zm1bzkwbw0000gn/T/opencode/mac-tunnel-connection-light.png`。
- 截图：`/private/var/folders/gx/qmlmrzyn4kb33r1zm1bzkwbw0000gn/T/opencode/mac-tunnel-dark-final.png`。

## 未覆盖与已知限制

### 目录折叠与复制反馈增量回归（2026-11-24）

- 环境：Ego Lite，`http://127.0.0.1:1420/`，TaskSpace 9 / p1，1200×706，浏览器无原生后端预览；所有测试目录数据仅在页面内存中。
- 通过：新增目录自动展开；两个目录独立收起/展开；启用开关不触发展开；Enter可展开；收起再展开后名称、子路径及本地路径保留；切换页面返回后已有目录默认收起。
- 通过：真实点击复制本地地址，显示底部“地址已复制”轻提示，无复制成功长条；提示自动消失，连续复制后从最后一次操作重新计时，实测约2046ms。
- 通过：亮色展开/轻提示、暗色两个收起卡片截图可读；1200px无横向溢出；预览保存仍显示明确原生后端错误，不假报成功。
- 通过：`bun run build`、`bun run tauri build --debug`；更新原路径开发包，41.36MiB。未操作运行中的客户端或隧道。
- 证据：`/private/var/folders/gx/qmlmrzyn4kb33r1zm1bzkwbw0000gn/T/opencode/mac-tunnel-copy-toast-light.png`、`/private/var/folders/gx/qmlmrzyn4kb33r1zm1bzkwbw0000gn/T/opencode/mac-tunnel-mappings-collapsed-dark.png`。
- 未覆盖：原生WebView剪贴板、公网地址复制、密钥及部署脚本复制的原生端到端场景；剪贴板拒绝写入错误场景；长名称压力和密码输入收起保留专项测试。后三处复制复用同一逻辑，但不能据此标记全部原生交互通过。

- 原生目录选择、真实IPC全流程、托盘隐藏/退出、登录启动尚未实测，不能标为通过。
- 未连接真实服务器，不修改现有20004/18788隧道；真实公网、断网/睡眠重连、指纹变化、SSH子进程回收仍需集成验证。
- 加密私钥/ssh-agent支持、配置schema迁移/导出、字段级错误、持久化主题选择未完成；现有仅启动时读取系统主题并允许手动切换。
- 前端尚无独立单元测试；SSH可替换驱动/退避状态机测试未完成。
- 路径检查拒绝现有符号链接，但仍需以描述符级打开消除检查与文件读取之间的竞态；公开目录内容应可信且勿由其他进程并发替换为敏感文件链接。
- 密码更新停止旧服务，需要手动重启，不是无中断热更新。

## 结论

### 服务端脚本简化（2026-11-25，取代前版交互流程）

- 按用户要求改为顶部参数、无交互直接创建，保留账号/配置/端口冲突保护；已配置目标跳过凭据变更。可选账号密码但SSH禁用密码登录，只用公钥；不保存用户真实凭据。
- Nginx仅输出片段和连接后回环/公网/实例标识验证方法。移除yes和独立--check，之前交互用法不再适用。
- 本地3项测试通过，未执行真实Linux账号创建或SSH重载，未做Linux/OpenSSH7.4及公网实测。

### 服务端初始化脚本（2026-11-25）

- 完成`scripts/service.sh`：Linux/root保护、交互参数校验、专用账号保护、公钥校验/追加、端口占用保护、SSH候选配置校验/备份/失败恢复/重载，Nginx仅输出片段，提供`--check`及连接后验证步骤。每个非空操作行含注释。
- `python3 scripts/test_service.py` 3项通过：help/语法、操作行注释、提取生成的SSH配置并用本机OpenSSH语法及有效配置解析检查；仅临时生成测试主机密钥，无服务变更。macOS执行`--check`明确拒绝（退出1），未操作本机系统配置。
- 未覆盖Linux账号创建、systemd重载、OpenSSH7.4实际兼容、失败恢复全流程、真实Nginx及公网端到端。未连接/修改真实服务器。

### 紧凑目录及目录级LAN（2026-11-25）

- `bun run build` 通过，Rust 10项测试通过。新增旧配置LAN默认false、目录读取过滤隐藏/密钥/符号链接/越界；模拟真实连接来源验证未授权LAN目录404、允许LAN的编码文件200、密码401及健康端点404。此为后端测试，不是跨设备实测。
- Ego Lite `http://127.0.0.1:1420/`，TaskSpace 1 / p1，1200×707：新增目录默认LAN关闭；切换开关、折叠后行高54px且无横向溢出；浏览文件弹窗打开，预览环境明确报无原生后端，Escape关闭通过。
- 截图：`/private/var/folders/gx/qmlmrzyn4kb33r1zm1bzkwbw0000gn/T/opencode/mac-tunnel-compact-lan.png`。
- 未覆盖：真实目录树展开及文件复制、暗色弹窗、原生IPC、系统IP查询结果、跨设备防火墙及网页资源兼容性。本次未安装、未启动真实LAN共享、未修改服务器；原有TOCTOU限制保留。

### 取消旧版备份（2026-11-25）

- 按用户要求移除安装时ZIP压缩逻辑。旧包仅临时移入本次暂存目录，安装失败恢复、成功删除，不保留额外应用或压缩包。此前已存在的压缩文件未删除。
- 脚本语法、help和dry-run检查通过；本次未实际覆盖安装。

### 覆盖安装重复图标修正（2026-11-25）

- 定位4个同标识应用：固定安装、Applications中的旧备份、项目debug/release产物。
- 脚本改为ZIP备份，安装成功删除本次暂存旧包，失败可恢复；成功撤销开发产物注册、注册固定安装版。bash语法及dry-run通过；修改后的完整安装流程尚未实测。
- 实际将确认标识的一个旧备份.app压缩到install-backups，原备份移到废纸篓；撤销两份开发包注册、注册当前安装版成功。当前安装包未替换，应用数据与密钥未修改。未重置Launchpad、未重启Dock，启动台图标刷新仍待用户确认。

### 密钥选择、位置与全局Toast增量（2026-11-24）

- 前端 `bun run build` 通过；Rust 8项测试通过，新增空ID/非法ID/缺失文件错误区分，以及临时生成Ed25519密钥并读取后生成部署脚本的测试。不读取用户已有私钥。
- `./scripts/app_install.sh --keep-version --no-install` 通过，保持用户当前0.1.1版本，开发包约41.41MiB；没有覆盖真实安装或停止客户端。
- Ego Lite：`http://127.0.0.1:1420/`，TaskSpace 10 / p1，1200×706。通过：部署助手未选密钥显示可关闭错误Toast；预览保存失败走全局错误Toast，无顶部长条。
- 隔离组件（明确标注模拟数据）通过：暗色显示私钥/公钥完整路径，点击复制私钥路径显示成功Toast并自动消失，1200px无横向溢出。该测试不证明原生IPC及用户真实路径已验收。
- 截图：`/private/var/folders/gx/qmlmrzyn4kb33r1zm1bzkwbw0000gn/T/opencode/mac-tunnel-key-selection-toast.png`；`/private/var/folders/gx/qmlmrzyn4kb33r1zm1bzkwbw0000gn/T/opencode/mac-tunnel-key-paths-dark-fixture.png`。
- 未覆盖：原生生成/导入后的自动选中与持久化、实际已安装客户端路径展示及复制、真实公网部署。后端测试与模拟页面不替代原生验收。

### 本机打包安装脚本（2026-11-24）

- `bash -n scripts/app_install.sh`、帮助及dry-run通过。
- 隔离临时目录配合模拟Tauri产物：patch递增、指定版本、非法版本不写入、dry-run不写入、带空格安装目录、旧包备份、运行中拒绝安装、冲突参数拒绝均通过；模拟测试不等于真实安装验收。
- 真实执行 `./scripts/app_install.sh --keep-version --no-install` 通过，含vue-tsc/Vite和Tauri debug打包；读取产物Info.plist核对版本0.1.0成功，产物约41.41MiB。
- 未修改实际版本号、未替换 `~/Applications` 下真实客户端、未自动启动应用；真实安装/启动、release构建尚未通过本脚本实测。
- 后续按用户要求改为：打包暂存成功后自动请求旧客户端退出，再覆盖安装；最多等待15秒，失败中止、不强杀。更新后语法检查及dry-run通过，停止逻辑隔离模拟覆盖未运行、退出成功、请求失败、进程检查失败、超时中止；未实际停止真实客户端，macOS应用退出及其托管服务回收仍待原生验证。此前“运行中拒绝安装”模拟结果对应旧行为。

可用于继续本地开发和原生试用，不应视为全部计划完成或可正式公网发布的安全验收。
