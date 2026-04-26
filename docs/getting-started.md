# Getting Started

## Install

For market data:

```toml
[dependencies]
alpaca-data = "0.25.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

For trading:

```toml
[dependencies]
alpaca-trade = "0.25.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

For time and calendar semantics:

```toml
[dependencies]
alpaca-time = "0.25.1"
```

For option semantics:

```toml
[dependencies]
alpaca-option = "0.25.1"
```

For the high-level composition layer:

```toml
[dependencies]
alpaca-facade = "0.25.1"
```

For the mock server:

```bash
cargo install alpaca-mock
```

## First Client

```rust
use alpaca_data::Client;

let client = Client::builder()
    .credentials_from_env()?
    .build()?;
# let _ = client;
# Ok::<(), alpaca_data::Error>(())
```

```rust
use alpaca_trade::Client;

let client = Client::builder()
    .credentials_from_env()?
    .base_url_from_env()?
    .build()?;
# let _ = client;
# Ok::<(), alpaca_trade::Error>(())
```

## Next Steps

- Read [Installation](./installation.md)
- Read [Authentication](./authentication.md)
- Review [Project Structure](./project-structure.md)
- Browse [Reference Index](./reference/index.md)
- Read [Mock Server](./mock-server.md)
- Read [Testing Guide](./testing.md)
- Read [Market Data API Coverage](./api-coverage/market-data.md)
- Read [Trading API Coverage](./api-coverage/trading.md)
- Read [Release Checklist](./release-checklist.md)
