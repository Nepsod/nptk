# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0](https://github.com/maycoon-ui/maycoon/compare/maycoon-core-v0.4.0...maycoon-core-v0.5.0) - 2025-05-01

### Added

- Render Context for AppContext
- Diagnostics for AppContext
- ActionSignal signal
- is_locked method for RwSignal
- RwSignal shortcut
- RwSignal based on the RwLock

### Fixed

- Remove unnecessary Arc's
- Unnecessary  Arc

### Other

- Update runner() method
- Update context.rs
- Re-organize features

## [0.4.0](https://github.com/maycoon-ui/maycoon/compare/maycoon-core-v0.3.2...maycoon-core-v0.4.0) - 2025-04-29

### Added

- Global State Management

### Fixed

- Default Font Selection

### Other

- Remove parking_lot dependency
- Update taffy to 0.8.1

## [0.3.1](https://github.com/maycoon-ui/maycoon/compare/maycoon-core-v0.3.0...maycoon-core-v0.3.1) - 2025-04-19

### Other

- Temporarily fix font issues
- Fix cargo asset packaging

## [0.3.0](https://github.com/maycoon-ui/maycoon/compare/maycoon-core-v0.1.0...maycoon-core-v0.3.0) - 2025-01-26

### Other

- Fix typo
- Fix clippy lints
- Fix `clippy::doc_markdown` lints
- Fix updating vello
- Update taffy and winit
- Implement component architecture
- Add size info
- Add Task Runner
- Make self in widget_id immutable
- Add init_threads config parameter
- Update dependencies
- Merge pull request [#28](https://github.com/maycoon-ui/maycoon/pull/28) from waywardmonkeys/update-to-vello-0.3
- Add way to load system fonts
- Replace dashmap with indexmap

## [0.1.0](https://github.com/maycoon-ui/maycoon/releases/tag/maycoon-core-v0.1.0) - 2024-10-04

### Other

- Update config.rs
- Fix non-windows compat
- Add workspace keys
- Add EmptyState
- Rename crates and rework state value
