# Alpaca Rust Spec Index

This directory stores crate-scoped specification documents for the shared Alpaca workspace.

The goal is to keep one durable place for:

- canonical public API definitions
- shared models consumed by Rust and TypeScript
- semantic rules that must stay aligned across crates

## Layout

- `alpaca-time/`
- `alpaca-option/`
- `alpaca-facade/`
- `alpaca-core/`
- `alpaca-data/`
- `alpaca-http/`
- `alpaca-mock/`
- `alpaca-trade/`

Each crate directory may contain:

- `api/` for public or internal API contracts
- `models/` for shared structural models
- `semantics/` for behavior and domain rules

## Current Migration Status

- `alpaca-time` carries the migrated time-and-calendar spec content from the legacy sibling workspace
- `alpaca-option` carries the migrated provider-neutral option core spec content from the legacy sibling workspace
- `alpaca-facade` carries the migrated Alpaca adapter spec content from the legacy sibling workspace
- the remaining crate directories currently provide placeholders and can be expanded as their contracts stabilize
