# Fixtures

This directory stores canonical JSON fixtures shared by the `alpaca-time` and
`alpaca-option` test suites.

Current fixture groups:

- `calendar/`, `display/`, `dst/`, `expiration/`, `parsing/`, `range/`,
  `session/`: market time and trading-calendar reference cases used by
  `alpaca-time`.
- `support/`: option analytics, contracts, pricing, Greeks, payoff, and URL
  reference cases used by `alpaca-option`.
- `layers/` and `catalog.json`: option fixture manifests that define the test
  layers and expected coverage for `alpaca-option`.

Fixture files are versioned test inputs. Update them only when the underlying
reference contract or expected behavior changes.
