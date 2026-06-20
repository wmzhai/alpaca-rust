# Examples

`alpaca-rust` does not currently publish a broad standalone example catalog.
It does include a focused trade-mainline example for the mock-backed lifecycle
path.

Use these guides as the practical starting points instead:

- [Getting Started](./getting-started.md)
- [Authentication](./authentication.md)
- [Mock Server](./mock-server.md)
- [Trade Mainline](./trade-mainline.md)
- [Reference Index](./reference/index.md)

Run the mock lifecycle example with:

```bash
cargo run -p alpaca-trade --example mainline_mock_lifecycle
```

The public workspace is intentionally documented around crate entry points,
resource guides, and a small number of lifecycle examples rather than long
standalone tutorial programs.
