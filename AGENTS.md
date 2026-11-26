# 随享项目开发指引

- 应用已更名“随享”，当前系统目录`/Applications/随享.app`为0.1.4；旧用户目录0.1.2已按标识核验并删除。内部identifier `cn.rutang.mac-tunnel`、可执行文件mac-tunnel、数据位置均保持兼容。旧文档安装路径仅作历史，不再代表当前安装产物。
- `scripts/app_bundles.py`在限定目录按Info.plist核验身份、拒绝危险符号链接并去重；安装成功后真正删除构建.app（不是只lsregister注销），避免再次扫描。`python3 scripts/test_app_install.py`有6项回归。首次安装及再次同版本覆盖已验收：仅随享.app和一条LaunchServices注册。直接Tauri打包/--no-install仍可能产生副本，不保证任意手工复制；不能宣称启动台视觉已验收。

本文件用于后续开发快速恢复上下文。更新功能或规划后，请同步维护本文件、README 和验收记录；不要把规划当成已完成功能。

## 项目定位与当前阶段

- 项目目录：`/Users/xuyifei_coco/Documents/tools/mac_tools/ssh_mac_server`。
- macOS 桌面工具，将多个本地目录通过一个 SSH 反向隧道映射到公网；新电脑不依赖本地 Nginx。
- 技术栈：Tauri 2、Vue 3 Composition API + TypeScript、Vite、Rust、系统 OpenSSH，包管理使用 Bun。
- 当前为 **0.1.0 开发预览阶段**：前后端主体已实现并生成 debug `.app`，不是正式发布或完整安全验收版本。
- 用户已批准整体设计与开发，无需再次询问是否开始；涉及新增范围、真实服务器变更或部署仍须获得相应授权。

## 开发前必读

1. 检查 Git 工作区，保留用户已有修改，不动父仓库其他项目。
2. 项目规则存在时先读 `tmp/project.md`。
3. 阅读 `README.md` 和 `acceptance-report.md`，了解实际可用范围及未验证项。
4. 阅读设计 `docs/superpowers/specs/2026-11-24-mac-tunnel-design.md` 和计划 `docs/superpowers/plans/2026-11-24-mac-tunnel.md`。
5. 存在 `.codegraph/` 时，定位、理解代码优先使用 CodeGraph；不要自行重建索引。
6. Vue 开发加载 `vue-best-practices`；界面参考 `ui-ux-pro-max`，保持紧凑蓝灰桌面工具风格、系统字体和清晰状态。

## 当前已实现的主体功能

- 浏览器共享目录样式（源码，未安装）：`listing.rs/listing.css`服务端统一首页/目录列表，蓝灰系统字体、彩色SVG、类型/大小、目录优先、前缀保留的面包屑/返回、空态、亮暗/手机适配；手机密码标记保留，长名完整换行。files.rs沿用安全筛选与认证/LAN/index优先；列表HTML加CSP/no-store/nosniff，无脚本/CDN。Rust串行37项、bun18项与构建通过；Ego TaskSpace4独立临时fixture验收首页→子目录→返回/Markdown、空态和桌面/390px，无横溢出；已finish/停止测试服务，未替换20005。

- TIFF 兼容（源码，未安装）：`image_preview.rs`通过image(TIFF/PNG)将本地`.tif/.tiff`默认转PNG第一帧；files.rs既有路径/认证/LAN校验后分派，无需改Markdown。raw=1原字节/download=1原图附件保留Range，预览忽略Range/HEAD空正文。16MiB输入、8192宽高/16M像素、128MiB库解码预算（非硬进程上限）、2个并发忙时503，损坏422/超限413，无远程代理/缓存/落盘。Ego独立回环端口只读验证用户ceshi.md的image.tiff正常655×312；20005安装服务未替换。example可显式传只读共享目录参数。

- Markdown 预览（源码，未安装）：`files.rs` 在既有路径/认证/LAN 校验后，将 `.md/.markdown` 默认交给 `markdown.rs` 服务端渲染，样式 `markdown.css`；pulldown-cmark + syntect(default-fancy) + ammonia，无 CDN/脚本。支持标准 Markdown 图片、表格/任务列表/删除线、代码高亮；移除原始 HTML，允许 HTTP/HTTPS 图片，保留相对链接和公网前缀。`?raw=1` 文本、`?download=1` 原文件下载均沿用 ServeFile/Range；预览不按原文件 Range 截取。有界读取2MiB、UTF-8检查；代码块>64KiB降级普通代码。独立 example `markdown_preview` 使用临时目录/随机回环端口，无SSH。Ego图片加载/多色高亮/源码/下载/亮色桌面及390px暗色手机通过，未更新20005当前安装服务，未真实公网验收。

- 新生成 SSH 密钥的 `-C` 注释改为 `suixiang-xyf-key`；已有/导入密钥不改。临时生成密钥测试验证公钥注释通过，需重新构建安装生效。

- 共享工作台优化（源码，未安装）：概览按本地、SSH、公网展示下一步；连接配置先本地，再可选公网与高级设置。目录先选本地路径，地址按本机、局域网、公网区分，共用已保存快照与 `shareBlockReason`/`fileLink`。无变化保存不停止服务；运行中保存配置或密码明确提示会停止共享，完成后可手动重新启动本地服务。相关旧版按钮位置以此条和 README 新章节为准。

- 连接表单修复长密钥select的隐式grid最小宽度：fields标签列minmax(0,1fr)、控件限宽。scroll-region仅纵向auto、不预留空滚动槽，6px安全内边距；输入/select焦点为主题色2px外框+1px偏移，避免裁边。连接表单压缩间距并移除空反馈占位；矮窗口后台运行仍需纵向滚动，禁止一律隐藏。Ego亮暗/长选项/1200×707及1000×700无横溢出、1200×1000无滚动通过；尚未安装原生验收。

- 目录映射本地打开/复制改用系统lan_addresses查询的私网IPv4，单IP直达、多IP按需选择；共用fileLink编码，检查保存配置对应端口/路径/启用/LAN及本地运行状态。LAN复选框与标签8px紧凑排列、说明独立弱化。浏览器模拟IPC验证通过，尚未安装/跨设备验收；概览127.0.0.1入口保持不变。

- 文件浏览弹窗使用Lucide彩色SVG与36px列表行、28px独立“局域网/公网隧道”复制按钮（桌面密集操作取舍）；不常驻地址，仅复制LAN且多网卡时显示选择。`src/utils/fileLinks.ts`封装文件分类和两渠道独立校验/编码，`tests/file-links.test.ts`覆盖。复制失败仅Toast，不堆叠常驻错误。公网需可访问，LAN需目录授权及有效IP，两者都需本地服务启动。

- 目录映射收起行约54px，提供独立目录树dialog，按已保存映射ID懒加载子项、文件复制链接；新组件 `DirectoryBrowser.vue`，后端 `browser.rs`。公网链接需状态“可访问”；未配域名则选择系统私网IPv4。实际跨设备访问尚未验收。
- 每目录 `allowLan` 默认false，旧配置兼容。启用LAN目录存在时监听0.0.0.0，否则127.0.0.1；HTTP以真实TCP来源隔离非回环请求，只能列出和访问允许LAN目录，健康端点不向LAN公开，不信任代理头。修改需保存和重启。

- 左导航：概览、目录映射、连接配置、密钥管理、部署助手、运行日志。
- 一个连接配置承载多个目录映射；可新增、修改、启停和移除映射，移除不删除磁盘资料。
- 本地服务只监听 `127.0.0.1`，可独立启动，不依赖 SSH；`/` 展示启用目录入口，`/docs/` 访问对应目录。
- 上述回环监听为默认策略；开启目录级LAN后的监听与隔离规则见本节开头。
- 同一文件服务也识别带公网基础前缀的地址，如 `/my_mac/docs/`；本地与公网认证规则一致。
- 只读 GET/HEAD、index.html 优先、安全目录列表、文件流式响应与 Range；拒绝点文件、敏感密钥文件、路径穿越和现有符号链接。
- 每目录 HTTP Basic 用户名密码，Argon2id 哈希保存，所有子资源统一认证；密码至少8位，公网必须建议 HTTPS。
- 配置保存到应用数据目录，原子替换、配置文件600；私钥目录700、私钥600；前端不回传真实密码哈希。
- 系统 `ssh-keygen` 生成 Ed25519 或导入无口令私钥，展示公钥及指纹，不上传私钥。
- 部署助手生成公钥追加脚本与 Nginx 片段，仅展示/复制，不自动执行服务器变更；目标低权限账号需预先存在。
- 扫描主机指纹，用户通过独立渠道核对后确认信任，使用应用专用 known_hosts。
- SSH 参数数组执行，独立配置文件、严格主机校验、回环反向转发、心跳与断线退避重连；主机身份校验失败停止重试。
- 本地、SSH进程、公网可达性分别展示；公网探测核对健康端点的应用实例标识，不能把进程存在当公网可用。
- 关闭窗口隐藏到菜单栏；菜单栏提供显示窗口与停止服务退出；可选登录启动，仅启动应用，不自动公开目录。
- 启动时读取系统主题，可手动切换亮暗；未保存修改切换页面有提示。
- 目录映射已有项默认收起、新增自动展开，各项独立展开且收起保留输入；地址、公钥、部署脚本复制成功使用共享2秒浮动轻提示，失败仍保留明确错误。
- 所有操作反馈统一使用 `AppToast.vue` / `useToast.ts`（success 2秒、info 5秒、error手动关闭），不使用顶部长条；预览环境说明保留普通文字，未保存离开确认保留。
- 生成/导入密钥后自动选入当前连接草稿；加载时未选密钥且只有一个可用密钥则默认选择，需保存配置。空密钥选择与不存在密钥给出明确错误，不再将未选择误报为密钥ID非法。
- 密钥管理显示应用托管的私钥/公钥完整路径并支持复制路径，绝不向前端返回私钥内容。
- 应用图标为“文件夹向外连接”图形；侧栏和应用包使用彩色 SVG 生成的 PNG/ICNS，菜单栏使用单独的透明单色模板图标，由 macOS 按菜单栏主题着色。

### 重要运行语义

- macOS Dock点击由RunEvent::Reopen调用show_main_window恢复已有窗口（含隐藏/最小化），不同于第二进程的single-instance回调。仅源码编译及串行Rust26项通过，未安装/原生点击验收；并发全套再次出现既有监听端口重绑AddrInUse，串行全通过，保留未定原因。

- Builder首个插件为tauri-plugin-single-instance（2.5.5），在setup前阻止重复实例；回调及托盘打开复用show_main_window(show/unminimize/set_focus)。应用单实例，不禁止WebView/SSH合法子进程，不扫描/强杀旧版本进程；升级前需正常退出旧实例。macOS Rust26项、bun18项及构建通过；Windows实机多开/隐藏/最小化唤起未验收。

- Windows 子进程统一通过 platform::background_command/background_async_command 设置 CREATE_NO_WINDOW 和默认 stdin null，覆盖 SSH、keygen/keyscan、ipconfig、打开地址 cmd；扫描指纹管道显式覆盖 stdin。SSH 退出记录退出码，远端转发失败提示多设备端口冲突，不自动停止其他设备。macOS Rust26项、bun15项及构建通过；Windows专属两项进程回归接入构建工作流，尚未在Windows执行，不宣称断线根因或原生弹窗验收完成。

- `orphan_ssh.rs`为连接前孤立SSH恢复：macOS sysctl读真实argv，必须完整匹配实际Command（包括专用密钥/known_hosts及全部参数）；同用户且PPID1才TERM，信号前复核身份/argv，最多等5秒；其他父进程报冲突，失败停止连接。Runtime首次spawn前执行，测试假程序跳过。身份取UID/PPID/lstart，不包含易变stat。当前只源码与22项Rust回归，未安装/未真实清理验收，禁止宣称可清理任意端口占用。

- 概览本地入口必须复用openAddress/open_url，禁止仅target=_blank（桌面WebView可能无外部窗口响应）。2026-11-25清理确认的升级前孤立64677后，当前随享自动重连，公网实例匹配；不是新版退出泄漏证据。链接源码修复尚未覆盖安装，原生点击仍待验收。

- 公网健康检查：每次SSH进程启动后约3秒首检，之后检查完成起600秒再检；右上角“验证公网”通过check_public命令向同一隧道任务排队，结果返回前不持有Runtime锁，停止/断线可取消。手动检查完成重置自动期限。界面status轮询、SSH心跳不变；公网显示是上次结果而非持续保障。

- DeploymentPanel确认信任成功后绿色勾选/禁用；失败不变绿，读取或主机/端口改变会清除确认，revision防迟到响应误标新主机。此为面板内反馈，尚未从known_hosts恢复跨页面/重启的可信显示。

- `Runtime::stop_tunnel/stop/connect`现为async，调用必须await；oneshot停止信号由持有Child的任务接收，kill().await回收后join返回，禁止正常路径直接abort隧道任务。退避/健康探测均可取消；本地HTTP取消后await。托盘退出只请求exit，统一ExitRequested先prevent_exit，异步stop成功置exit_ready后再次exit，exiting阻止启动/连接/配置修改。关闭窗口仍隐藏；Drop只尽力发送停止信号，强杀/崩溃不保证。假进程回归覆盖回收、重复连接、退避、健康请求、Drop及监听释放；原生退出与更新安装尚未实测。

- SSH `UserKnownHostsFile` 的值必须按OpenSSH配置语法双引号引用并转义，不能只依赖argv分隔；macOS `Application Support`含空格，否则会误报主机校验失败。保留严格校验。概览通过`src/utils/statusTone.ts`统一异常红/运行与可达绿/连接重连黄/空闲中性，不能用SSH进程绿色替代公网健康验证。

- 桌面布局由`src/desktop-layout.css`约束：窗口根节点不滚动，左导航/右顶栏固定；`workspace-panel`、`scroll-region`、`workspace-heading/footer`定义分区，表单/目录/密钥内部滚动，保存固定底部；脚本限高240px、日志独立滚动。新增面板需保留min-width/min-height:0，禁止长文本撑宽窗口。弹窗仅文件列表滚动。

- 概览未保存时提供“保存并启动本地服务”，串行保存成功后启动；保存失败保留草稿且不启动。阶段文案及防重复提交由`src/composables/useServiceControls.ts`负责，`tests/service-controls.test.ts`覆盖（`bun test`）。服务运行时禁用重复启动，dirty时提供明确会停止服务的保存入口。公网按钮展示未满足条件，本地不依赖服务器/密钥；首次指引分本地与可选公网。

- 默认本地端口 `20005`，服务器转发端口 `18790`，公网前缀 `/my_mac`；服务器和域名默认不预填真实环境。
- 启动本地服务与连接公网隧道是独立按钮，连接隧道前必须启动本地服务。
- 单独断开隧道保留本地服务；停止全部同时停止两者。
- 保存配置或修改密码会停止旧服务，需手动重新启动，当前不是无中断热更新。
- `bun run dev` 仅为浏览器界面预览，无原生后端，操作必须明确报错，不得伪装真实服务成功。

## 代码结构

| 路径 | 职责 |
| --- | --- |
| `src/App.vue` | 应用导航、概览、状态轮询、配置与操作入口 |
| `src/components/ConnectionPanel.vue` | 连接配置、登录启动设置 |
| `src/components/MappingsPanel.vue` | 目录映射、访问保护、地址打开/复制 |
| `src/components/AppToast.vue`、`src/composables/useToast.ts`、`src/composables/useCopyFeedback.ts` | 全局浮动提示、计时及剪贴板反馈 |
| `src/components/KeysPanel.vue` | 密钥生成、导入和公钥展示 |
| `src/components/DeploymentPanel.vue` | 授权/Nginx脚本、主机指纹确认 |
| `src/api.ts`、`src/types.ts` | 原生调用适配和前端类型 |
| `src/styles.css`、`src/controls.css` | 布局主题及系统按钮样式修正 |
| `src-tauri/src/lib.rs` | Tauri命令、应用状态、托盘与窗口生命周期 |
| `src-tauri/src/config.rs` | 配置模型、校验、持久化 |
| `src-tauri/src/files.rs` | 目录路由、认证、安全路径和文件服务 |
| `src-tauri/src/keys.rs` | 系统密钥操作 |
| `src-tauri/src/deployment.rs` | 部署脚本生成 |
| `src-tauri/src/runtime.rs` | 本地服务与SSH进程、重试、状态探测 |
| `src-tauri/tauri.conf.json`、`src-tauri/capabilities/` | 应用与最小权限配置 |

## 后续规划与优先级

### P0：安全与生命周期收尾

- 文件路径当前先检查再交给文件服务打开，存在 TOCTOU 竞态；需改为描述符级安全打开，防止检查后被替换为符号链接导致逃逸。未解决前，仅分享可信且不会被其他进程恶意替换的目录。
- 正常停止/退出等待子进程回收已实现并有假进程回归；继续实测Tauri原生Cmd+Q/托盘/更新安装退出和真实远端端口释放，调查并发测试一次本地端口立即重绑失败（随后全套及5轮runtime回归通过），完善异常退出策略。
- 增加可替换SSH进程驱动，测试首次失败、远端端口占用、退避、主动停止、配置切换、指纹变化及日志脱敏。
- 检查主机指纹扫描/确认与配置切换并发行为，确保不会信任错主机或意外替换既有信任。

### P1：功能与自动化测试完善

- 扩展 HTTP 测试：HEAD、UTF-8/空格、双编码、目录列表转义、隐藏文件、404、密码修改/取消及多目录隔离。
- 支持加密私钥及 ssh-agent 已解锁密钥，不能保存私钥口令。
- 补配置 schemaVersion、迁移、损坏配置恢复及安全导入导出；导出排除私钥、密码哈希和凭据。
- 补字段级错误、加载与禁用状态、系统/亮/暗主题持久化，以及前端独立单元测试。
- 完善部署助手的端口冲突、低权限账号设置和授权撤销说明，兼容旧OpenSSH。

### P2：完整验收与交付

- 原生实测：IPC、目录/密钥选择、真实本地访问、托盘隐藏/恢复/退出、登录启动。
- 经授权使用独立端口及测试目录验证公网、断网恢复、睡眠唤醒，不影响既有服务。
- 完成 release 构建、目标架构说明、图标原生呈现检查、签名/公证和分发流程；没有实际完成不得宣称已签名或可无提示分发。

## 构建与验证命令

- 0.1.4系统安装只读诊断：arm64匹配，codesign/spctl报code has no resources but signature indicates they must be present，仅linker adhoc签名、无资源封装，带Chrome隔离标记。GitHub macOS打ZIP前增加完整bundle临时签名/严格验签、解压回读验签；未远端执行，不等同Developer ID/公证，不擅自清除隔离或修改安装包。

- 仅同步三处版本用 `bun run version:bump`（scripts/bump-version.ts），默认patch+1，支持 `--version X.Y.Z`/`--dry-run`；不打包、不安装、不发布，Cargo.lock由后续Cargo更新。已先增版再本地打包时用package.sh --keep-version避免加两次。

- Windows Release 附件现为 `suixiang_<版本>_windows_x64.zip`，内含原始 NSIS exe，解压后安装，非便携版；上传前 PowerShell 解压回读并校验 SHA-256。仅上传 ZIP，不删除历史 Release 已有 exe；macOS zip不变。Windows归档实测待GitHub runner执行，不把ZIP封装当作损坏根因修复。

- GitHub `.github/workflows/build.yml`：手动 Run workflow 与推送 `v*` 标签均在双平台构建成功后发布 Release（Windows exe/macOS zip），同时保留 artifact。`scripts/release-metadata.ts` 校验三处版本一致与标签匹配，手动以源码版本生成标签，不自动增版；同版本重跑更新附件但不移动已有标签。只有 release job 可写仓库。未配置签名/公证，远端构建发布尚未实测；本地 `bun test tests/release-metadata.test.ts` 回归。

- `scripts/service.sh`：Linux/root非交互准备脚本，参数集中顶部，账号/可选密码/公钥预填后直接运行；禁止填入私钥或提交真实密码。默认mac-tunnel；目标配置标记存在跳过创建，已有账号无标记拒绝接管。无yes确认，无独立--check。Nginx仅输出片段，端到端验证须Mac连接后核对回环及公网健康实例标识。项目开发时不直接在真实服务器执行。`python3 scripts/test_service.py`为本地3项无部署检查，不能替代Linux/OpenSSH7.4验收。

- 安装覆盖固定的 `/Applications/随享.app`，不保留旧版或ZIP备份；仅替换期间临时保留旧包以便失败恢复，成功即删除。禁止在Applications生成备份.app以免Launchpad重复。安装成功撤销并移除debug/release及应用目录内核验身份的重复.app，再注册安装版。此前归档的ZIP不自动清理。

- 本机测试脚本 `scripts/app_install.sh`：默认patch版本+1、release打包并安装到 `/Applications`；支持 `--version X.Y.Z`、`--keep-version`、`--debug`、`--release`、`--no-install`、`--install-dir`、`--open`、`--dry-run`。无系统目录写权限时报错，不自动sudo或回退用户目录；仍扫描旧用户Applications核验去重。
- 版本同步与构建统一由 `scripts/package.sh` 负责（`bun run package` 或直接执行），默认 release 且默认 patch+1；只有显式 `--debug` 才生成 debug 包。安装脚本复用该入口，避免一次操作重复递增版本；同步三处版本，构建失败保留新版本。打包暂存成功后通过应用标识请求旧客户端退出，最多15秒，失败则中止覆盖；成功不保留旧包，不强杀、不清除应用数据、不操作独立手工隧道。客户端退出会中断其托管服务，运行前注意版本变更与目标安装覆盖。

```bash
bun install
bun run tauri dev
bun run build
bun run package
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
./scripts/package.sh --debug
```

- 开发界面：`http://127.0.0.1:1420/`。
- 默认打包产物：`src-tauri/target/release/bundle/macos/随享.app`；显式 `--debug` 时为 `src-tauri/target/debug/bundle/macos/随享.app`。本机架构，未做Developer ID签名/公证。
- Windows `.exe` 不能在 macOS 交叉编译（`ring` 需 MSVC/Windows SDK），使用 `.github/workflows/build.yml` 在 `windows-latest` 构建 NSIS；本地 Windows 可 `bunx tauri build --bundles nsis`。平台差异集中在 `src-tauri/src/platform.rs`，macOS 专属孤立 SSH 检查在其他平台自动跳过。
- 已记录结果：前端类型检查/构建通过；Rust 25项测试通过；Tauri debug打包通过（新默认 release 打包尚未在本轮实测）；Windows 原生编译/运行未实测。
- Ego Lite 已检查目录表单、预览错误反馈、未保存提示、亮暗及1200px无横向溢出；这不等于原生或公网验收。
- 页面回归必须使用 `ego-browser` 技能，先运行 `ego-browser skill`，再使用 `ego-browser nodejs`；同一目标复用TaskSpace，保留截图证据，完成后finish。不得改用Playwright或外部浏览器替代。
- 原生操作、公网连接、断网/睡眠、退出回收、自启动尚未实测。最新结果以 `acceptance-report.md` 为准。

## 现有环境与禁止误操作

- 2026-11-25经用户授权实测新服务器`82.156.234.208:22`，客户端公网源站`http://82.156.234.208`、前缀`/my_mac`、转发18790。OpenResty容器`1Panel-openresty-P5o4`为host网络，配置`/opt/1panel/apps/openresty/openresty/conf/default/00.default.conf`已有代理；备份后仅平滑reload，无内容改动。健康实例匹配、目录与PDF Range通过，未验收HTTPS。曾复现旧应用孤立SSH占18790造成重连等待，已只清理确认的孤立进程；退出回收缺陷未根治，不能泛杀ssh。旧服务器/手工18788仍不可动。

- 用户原有Mac手工隧道仍在使用，**不是本客户端托管实例**：公网 `/xyf_coco_mac/` → 服务器回环 `18788` → Mac回环 `20004`。
- 本机现有Nginx：`/opt/homebrew/etc/nginx/servers/ssh_sui_dao.conf`，分享 `~/Desktop/demo_html/`。
- 现有launchd：`~/Library/LaunchAgents/cn.rutang.xyf-coco-mac-tunnel.plist`。
- 现有私钥：`~/.ssh/xyf_coco_mac_tunnel_ed25519`。不得读取输出、上传、覆盖或默认导入。
- 服务器参考：`211.149.161.30:22000`，低权限隧道用户 `image-tunnel`，公网源站 `https://xyf.rutang.cn`。
- 手机使用云端18787及18789，Mac既有18788；开发默认18790，但真实连接前仍须确认占用及授权。
- 云端OpenSSH 7.4不支持新版PermitListen限制，不可生成假定支持的配置。
- 之前临时root公钥授权已撤销，不能假定仍有root访问权限，不索取或保存root密码。
- 不自动接管、停止或修改现有launchd/Nginx/手机服务；不删共享文件；不复制手机私钥；未经明确授权不执行服务器变更、git push或部署。
