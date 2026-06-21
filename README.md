# Codex-Shim

A tiny Rust Responses-to-Chat shim for Codex CLI to use OpenAI-compatible APIs.

It is intentionally narrower than RelayForge:

```text
Codex CLI / Responses client
-> local /v1/responses
-> OpenAI-compatible Chat Completions
```

> Not only Xiaomi MIMO, but also deepseek, longcat, GLM.
> `Codex-Shim` 不单单支持小米，更加支持Deepseek，美团的LongCat，智谱的GLM（均进行本地测试）。

## Features and Limitations / 功能与限制

[FEATURES.md](./FEATURES.md)

**Before use, please customize `base_instructions` in [mimo.json](./config/codex-cli/mimo.json).**

**使用前，请自行修改 [mimo.json](./config/codex-cli/mimo.json) 中的 `base_instructions`。**

### Recommended codex-cli version / 推荐的 codex-cli 版本

|    Project version / 项目版本 | codex-cli version / 搭配 codex-cli 版本 | Notes / 注释                    |
|--------------------------:|------------------------------------:|-------------------------------|
|                    v0.1.9 |                           `0.116.0` | Recommended / 推荐使用            |
|                    v0.1.9 |                           `0.119.0` | before Codex-cli change tools |
|                Preview UI |                           `0.130.0` | -                             |
|                Preview UI |                           `0.141.0` | Coming soon (in local test)   |

> v0.1.x will keep open source and do some code clean.

> v0.2.x some crates is my private crate. And use `tauri 2` for user UI. Support some features which `Not included` in v0.1.x.

```bash
npm install -g @openai/codex@0.116.0
```

### Screenshot

<table>
  <tr>
    <td width="50%">
      <img src="./assets/screenshot1.webp" alt="Codex MiMo Shim screenshot 1" width="100%">
    </td>
    <td width="50%">
      <img src="./assets/screenshot2.webp" alt="Codex MiMo Shim screenshot 2" width="100%">
    </td>
  </tr>
</table>

## Scope

Supports:

```text
GET  /healthz
GET  /v1/models
POST /v1/responses
GET  /v1/responses/{response_id}
```

Explicitly handled but unsupported:

```text
POST /v1/responses/compact -> 501 compact_not_supported
```

`/v1/responses/compact` is a Codex remote-compaction control-plane request. This shim does not forward it to Xiaomi and does not emulate compaction in v0.1.9.

Main behavior:

```text
Responses input               -> Chat Completions messages
instructions                  -> system message
max_output_tokens             -> max_tokens
temperature / top_p           -> passthrough, or config defaults
tools:function                -> Chat Completions tools
Chat tool_calls               -> Responses function_call items
function_call_output          -> Chat tool message
previous_response_id          -> in-memory continuation
MiMo reasoning_content        -> stored and replayed when thinking mode is enabled
stream=true                   -> buffered Responses SSE
```

Not included:

```text
multi backend
legacy proxy
flight recorder
context footprint
cache
local MCP execution
Codex MCP config loading
provider routing
compact emulation
```

## Build

```bash
cargo build --release
```

## Configuration

Configuration is file-first.

Lookup order:

```text
1. --config /path/to/config.yaml
2. --config=/path/to/config.yaml
3. ./config.yaml
4. ./config.yml
5. ./config.json
6. built-in defaults
```

Environment variables are **not** the general configuration surface. Only API-key sensitive overrides are read:

```bash
MIMO_API_KEY
AK_XIAOMIMIMO_TKP
XIAOMIMIMO_API_KEY
```

All other options belong in `config.yaml` or `config.json`.

Copy the example config:

```bash
cp config/config.example.yaml config.yaml
```

Minimal `config.yaml`:

```yaml
server:
  host: 127.0.0.1
  port: 33300

upstream:
  base_url: https://token-plan-cn.xiaomimimo.com/v1
  api_key: NO-KEY
  model: mimo-v2.5-pro
  models:
    - mimo-v2.5-pro
  aliases:
    codex-auto-review: mimo-v2.5-pro
  fallback_unknown_model_to_default: true
  thinking:
    enabled: true

generation:
  default_temperature: 0.2
  default_top_p: 0.95
  default_max_output_tokens: 4096

http:
  request_timeout_secs: 300
  trust_env: false
  http2_prior_knowledge: false

behavior:
  access_log: true
  response_store_max: 1000
  forward_parallel_tool_calls: true

log:
  level: codex_mimo_shim=info,tower_http=warn
```

Run:

```bash
./target/release/codex-mimo-shim --config config.yaml
```

Or rely on `./config.yaml` auto-discovery:

```bash
./target/release/codex-mimo-shim
```

## Codex config

```toml
model = "mimo-v2.5-pro"
model_provider = "xiaomi-mimo"

[model_providers.xiaomi-mimo]
name = "Xiaomi MiMo via codex-mimo-shim"
base_url = "http://127.0.0.1:33300/v1"
wire_api = "responses"
api_key = "no-api-key"
experimental_bearer_token = "no-api-key"
```

## Access log

Access log defaults to enabled and does not print prompt/body/tool result.

Example:

```text
mimo-shim-in  unstream=true path=/v1/responses trace_id=TRC-7F4A1C2D...
mimo-shim-out unstream=true path=/v1/responses trace_id=TRC-7F4A1C2D... model=codex-auto-review provider_model=mimo-v2.5-pro model_route=configured_alias obs_usage=provider_usage obs_input_tokens=123 obs_output_tokens=45 obs_total_tokens=168 obs_cached_tokens=- obs_reasoning_tokens=- provider_ms=2380 total_ms=2415 status=200 error=-
```

When Codex triggers remote compaction, the shim returns a local unsupported response instead of a misleading upstream 502:

```text
mimo-shim-in  unstream=true path=/v1/responses/compact trace_id=TRC-7F4A1C2D...
mimo-shim-out unstream=true path=/v1/responses/compact trace_id=TRC-7F4A1C2D... model=codex-auto-review provider_model=- model_route=unsupported_codex_control_plane_request obs_usage=- obs_input_tokens=- obs_output_tokens=- obs_total_tokens=- obs_cached_tokens=- obs_reasoning_tokens=- provider_ms=0 total_ms=0 status=501 error="unsupported feature: /v1/responses/compact is a Codex remote compaction control-plane request; codex-mimo-shim does not emulate compact and does not forward it to Xiaomi"
```

Trace id resolution order:

```text
metadata.trace_id
trace_id
metadata.request_id
request_id
x-request-id
Rust-generated TRC-<uuid>
```

## Release artifacts

The GitHub Actions workflow builds directly from the source in this repository and publishes:

```text
- platform-specific archives for Windows, Linux, and macOS
- SHA256 checksums for release artifacts
- Cargo.lock and Cargo.lock.sha256
- build.rs and build.rs.sha256
- release.yml and release.yml.sha256
- platform-specific build-info JSON files
```

## Verify a release

See [VERIFY.md](VERIFY.md).

## Security policy

See [SECURITY.md](SECURITY.md).
