# Remote HTTP Agent

一个用 Rust 编写的 HTTP 反向代理，解决浏览器跨域、请求头限制等问题。

这是 [cors_reverse_proxy](https://github.com/wilinz/cors_reverse_proxy) Go 版本的 Rust 实现。

## 特性

- **完整的 CORS 支持**：自动处理预检请求和跨域头部
- **Bearer Token 认证**：保护代理端点
- **`tun-` 前缀头部转发**：灵活控制哪些头部转发到目标服务器
- **重定向处理**：3xx 响应转为 200，原始信息保存在 `tun-*` 头部
- **Set-Cookie 转发**：重命名为 `tun-set-cookie`，避免浏览器自动处理
- **流式传输**：高效处理大响应体
- **上游代理支持**：可配置 HTTP 代理
- **无控制台窗口**：提供 GUI 构建版本（Windows），启动不弹黑框

## 下载

从 [Releases](../../releases) 下载对应平台的预编译二进制：

| 文件名 | 平台 |
|--------|------|
| `remote_http_agent-windows-x64.exe` | Windows 64 位（带控制台） |
| `remote_http_agent-windows-x64-gui.exe` | Windows 64 位（无黑框） |
| `remote_http_agent-windows-x86.exe` | Windows 32 位（带控制台） |
| `remote_http_agent-windows-x86-gui.exe` | Windows 32 位（无黑框） |
| `remote_http_agent-macos-arm64` | macOS Apple Silicon |
| `remote_http_agent-macos-x64` | macOS Intel |
| `remote_http_agent-linux-x64` | Linux x86_64 |
| `remote_http_agent-linux-x64-musl` | Linux x86_64（静态链接，更好的兼容性） |
| `remote_http_agent-linux-arm64` | Linux ARM64 |
| `remote_http_agent-linux-arm64-musl` | Linux ARM64（静态链接） |
| `remote_http_agent-linux-armv7` | Linux ARMv7 |

## 快速开始

### 1. 配置

在可执行文件同目录下创建 `config.json5`：

```json5
{
  "listening": "0.0.0.0:10010",
  "token": "your-secret-token-here",
  // "http_proxy": "http://127.0.0.1:9000",
  "skip_tls": true
}
```

配置文件不存在时使用内置默认值直接启动。

### 2. 运行

```bash
./remote_http_agent
```

启动后会在当前目录生成停止脚本：
- Windows：`kill.bat`
- Linux/macOS：`kill.sh`

## 配置项

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `listening` | string | `0.0.0.0:10010` | 监听地址 |
| `token` | string | 随机 UUID | Bearer 认证 Token |
| `http_proxy` | string | `""` | 上游 HTTP 代理（可选） |
| `skip_tls` | bool | `true` | 跳过目标站点 TLS 证书验证 |

## API

### `GET/POST/... /proxy?url=<目标地址>`

转发请求到目标地址。

**请求头**：
```
Authorization: Bearer <token>
```

**示例**：
```bash
curl -H "Authorization: Bearer your-token" \
  "http://127.0.0.1:10010/proxy?url=https://api.example.com/data"
```

### `GET /lanip`

获取本机局域网 IP 地址。

```json
{"code": 0, "msg": "success", "ip": "192.168.1.100"}
```

### `GET /kill`

停止程序（等效于执行 `kill.bat` / `kill.sh`）。

```json
{"code": 0, "msg": "程序即将退出"}
```

## 头部转发规则

### `tun-` 前缀

发送 `tun-X-Custom-Header: value`，代理会以 `X-Custom-Header: value` 转发到目标服务器。

`tun-` 版本优先级高于同名默认头部，可用于覆盖默认白名单字段。

### 默认白名单（无需 `tun-` 前缀）

`Content-Type`、`Content-Length`、`Referer`、`User-Agent`、`Accept`、`Cookie`、`Accept-Encoding`、`Keep-Alive`

### 响应头处理

| 上游响应头 | 代理返回头 | 说明 |
|-----------|-----------|------|
| `Location` | `tun-Location` + `tun-Location-Proxy` | 重定向转为 200，URL 保存在此 |
| `Set-Cookie` | `tun-set-cookie` | 避免浏览器自动处理 |
| 3xx 状态码 | `tun-status` | 原始状态码 |

## 从源码构建

```bash
cargo build --release                        # 普通版（带控制台）
cargo build --release --features gui         # GUI 版（Windows 无黑框）
```

### 日志级别

```bash
RUST_LOG=debug ./remote_http_agent
```

## 项目结构

```
src/
├── main.rs      # 入口、中间件、路由
├── config.rs    # 配置加载
├── proxy.rs     # 代理核心逻辑
├── headers.rs   # 请求/响应头处理
├── auth.rs      # Bearer Token 验证
└── ip.rs        # 局域网 IP 获取
```

## 安全说明

- `token` 请设置为强随机值，不要使用默认值
- 未限制目标 URL，请在受信任网络环境中使用
- 生产环境建议 `skip_tls` 设为 `false`

## License

MIT
