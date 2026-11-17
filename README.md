# OXIDE C2

> Next-generation Rust Command & Control framework for authorized adversary emulation and red team research.

---

## Architecture

```
oxide/
├── oxide-common/     # Shared types, crypto (x25519 + ChaCha20-Poly1305), protocol
├── oxide-macros/     # Compile-time proc macros: encrypt_str!, dead_code!
├── oxide-server/     # Team server — axum WebSocket, HTTP/TCP listeners, SQLite
├── oxide-agent/      # Windows implant (EXE + cdylib reflective DLL)
└── oxide-tui/        # Operator terminal UI — ratatui, Canvas graph, OPSEC score
```

## Features

### Server
- Operator WebSocket interface (axum 0.7)
- HTTP and TCP agent listeners
- SQLite persistence (agents, tasks, loot)
- Per-build polymorphic payload generation

### Agent (Windows)
- Polymorphic build engine — unique XOR key per compilation via `build.rs`
- **Sleep masking**: Ekko, Foliage, Zilean
- **AMSI bypass**: patch `AmsiScanBuffer` at runtime
- **ETW bypass**: patch `EtwEventWrite` at runtime
- **Stack spoofing**: RAII `SpoofGuard`, RSP frame walk, ntdll gadget replacement
- **Sandbox evasion**: process count, uptime, artifacts, debugger, screen resolution
- Block non-Microsoft DLLs
- Indirect `NtDelayExecution` via dynamic `GetProcAddress`

### Post-Exploitation
| Module | Capability |
|---|---|
| `inject` | Shellcode injection (CreateThread / NtCreateThreadEx / APC Queue) |
| `token` | Token stealing / impersonation |
| `lsass` | LSASS minidump + SAM hive dump |
| `creds` | DPAPI decryption, Windows Vault |
| `recon` | Screenshot (GDI), port scan, process list |
| `persist` | Registry run key, scheduled task, service |
| `lateral` | PsExec, WMI lateral movement |

### TUI
- Canvas network graph — server at center, agents in orbit (colored by privilege)
- Live **OPSEC Score bar** (0–100%) — GREEN / YELLOW / RED thresholds
- 6-tab layout: Dashboard, Agents, Listeners, Sessions, Loot, Build
- Payload generator modal with full evasion option matrix
- Agent interact modal, listener creation modal
- Mouse support

### Proc Macros
- `encrypt_str!("literal")` — XOR-encrypts string at compile time, decrypts inline at runtime
- `dead_code!(N)` — injects N random stub functions to mutate binary hashes

## OPSEC Score Algorithm

| Feature | Points |
|---|---|
| Indirect syscalls | +15 |
| Stack spoofing | +15 |
| Zilean sleep mask | +20 |
| Foliage sleep mask | +15 |
| Ekko sleep mask | +10 |
| AMSI bypass | +10 |
| ETW bypass | +10 |
| Sandbox evasion | +10 |
| Module stomping | +10 |
| Encrypted comms | +10 |
| APC injection | +10 |
| NtCreateThreadEx | +5 |
| Block non-MS DLLs | +5 |
| DNS DGA | +5 |

Score ≥ 75 → LOW RISK · Score ≥ 45 → MODERATE RISK · Score < 45 → HIGH RISK

## Build

### Prerequisites
```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Windows cross-compile target
rustup target add x86_64-pc-windows-gnu
apt install -y mingw-w64
```

### Server
```bash
cargo build --release -p oxide-server
./target/release/oxide-server
```

### TUI
```bash
cargo build --release -p oxide-tui
./target/release/oxide-tui --server ws://127.0.0.1:7878/ws --token <token>
```

### Agent (Windows cross-compile)
```bash
cargo build --release -p oxide-agent --target x86_64-pc-windows-gnu
```

Payload generation is also available directly from the TUI Build tab.

## Protocol

- **Operator ↔ Server**: WebSocket, JSON-framed `OpMessage` / `SrvMessage`
- **Agent ↔ Server**: HTTP (`POST /register`, `POST /beacon/:id`) or TCP (4-byte length-prefix + JSON)
- **Crypto**: x25519 ECDH key exchange + HKDF-SHA256 session key derivation + ChaCha20-Poly1305 encryption

## Roadmap (Phase 8)
- [ ] DNS tunneling with DGA
- [ ] SaaS C2 (GitHub Gists / Discord webhooks)
- [ ] Environmental keying
- [ ] Secure self-destruct

---

> **For authorized red team engagements and security research only.**
