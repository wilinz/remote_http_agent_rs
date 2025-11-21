# Remote HTTP Agent

一个用 Rust 编写的 HTTP 转发器，用于解决 Web HTTP 不自由的问题，比如不允许修改某些头，不允许跨域。

这是 [cors_reverse_proxy](https://github.com/wilinz/cors_reverse_proxy) Go 版本的 Rust 实现。

## 特性

- ✅ **完整的 CORS 支持**：自动处理预检请求、跨域头部
- ✅ **Bearer Token 认证**：简单有效的认证机制
- ✅ **智能头部转发**：使用 `tun-` 前缀机制灵活控制头部转发
- ✅ **重定向处理**：自动处理 3xx 重定向，避免浏览器跨域错误
- ✅ **Set-Cookie 支持**：正确处理跨域 Cookie 设置
- ✅ **流式传输**：高效处理大文件
- ✅ **TLS 支持**：可选的 HTTPS 服务
- ✅ **代理链支持**：可配置上游 HTTP 代理

## 快速开始

### 1. 安装 Rust

确保已安装 Rust 1.70+：

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. 克隆并构建

```bash
git clone <this-repo>
cd remote_http_agent
cargo build --release
```

### 3. 配置

首次运行会生成配置模板：

```bash
cargo run
# 或
./target/release/remote_http_agent
```

这会创建 `config.temp.json5` 文件，将其重命名为 `config.json5` 并修改配置：

```json5
{
  "tls": false,
  "tls_cert": "",
  "tls_key": "",
  "listening": "0.0.0.0:10010",
  "token": "your-secret-token-here",  // 修改为自己的 token
  "http_proxy": "",
  "insecure_skip_verify": false
}
```

### 4. 运行

```bash
cargo run --release
```

## 使用方法

### cURL 示例

```bash
curl -i \
  -H "Authorization: Bearer your-secret-token-here" \
  -H "tun-Referer: https://example.com" \
  "http://127.0.0.1:10010/proxy?url=https://www.example.com"
```

### JavaScript/Fetch 示例

```javascript
const response = await fetch('http://127.0.0.1:10010/proxy?url=https://api.example.com/data', {
  headers: {
    'Authorization': 'Bearer your-secret-token-here',
    'tun-Content-Type': 'application/json',
    'tun-Referer': 'https://example.com'
  }
});

const data = await response.json();
```

## 头部转发机制

### `tun-` 前缀规则

只有以 `tun-` 前缀开头的头部会被转发到目标服务器（转发时去除前缀）。

**默认白名单**（无需前缀也会转发）：

- `Content-Type`
- `Content-Length`
- `User-Agent`
- `Accept`
- `Accept-Encoding`
- `Keep-Alive`

### 示例

**客户端发送**：
```
Authorization: Bearer token
tun-X-Custom-Header: value
tun-Cookie: session=abc123
Content-Type: application/json
```

**转发到目标服务器**：
```
X-Custom-Header: value
Cookie: session=abc123
Content-Type: application/json
```

## 响应头处理

### Location 重定向

重定向响应会被转换为 200 OK，原始信息保存在特殊头部：

- `tun-location`：原始重定向地址
- `tun-location-proxy`：转换为代理格式的地址
- `tun-status`：原始状态码

### Set-Cookie

上游的 `Set-Cookie` 头部会被重命名为 `tun-Set-Cookie`，避免浏览器自动处理。

## 配置选项

| 选项 | 类型 | 说明 |
|------|------|------|
| `tls` | bool | 是否启用 HTTPS |
| `tls_cert` | string | TLS 证书路径 |
| `tls_key` | string | TLS 私钥路径 |
| `listening` | string | 监听地址（如 "0.0.0.0:10010"） |
| `token` | string | Bearer 认证 Token |
| `http_proxy` | string | 上游 HTTP 代理地址（可选） |
| `insecure_skip_verify` | bool | 跳过目标站点 TLS 证书验证（仅开发环境） |

## 安全考虑

- **Token 认证**：确保 `token` 设置为强随机值，不要泄露
- **TLS 验证**：生产环境建议 `insecure_skip_verify` 设置为 `false`
- **SSRF 风险**：当前未限制目标 URL，请在受信任环境中使用
- **速率限制**：生产环境建议添加速率限制中间件

## 开发

### 运行测试

```bash
cargo test
```

### 开启日志

```bash
RUST_LOG=debug cargo run
```

## 项目结构

```
src/
├── main.rs         # 入口点和服务器配置
├── config.rs       # 配置管理
├── auth.rs         # Bearer Token 认证
├── proxy.rs        # 代理核心逻辑
└── headers.rs      # 请求/响应头处理
```

## 与 Go 版本的差异

- 使用 `axum` web 框架代替 `gin`
- 使用 `reqwest` HTTP 客户端代替 Go 的 `net/http`
- 完全异步实现，基于 `tokio` 运行时
- 类型安全的错误处理

## 许可证

MIT License

## 致谢

本项目是 [cors_reverse_proxy](https://github.com/wilinz/cors_reverse_proxy) 的 Rust 实现。
