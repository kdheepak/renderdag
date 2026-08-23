# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0](https://github.com/kdheepak/renderdag/compare/v0.3.1...v0.4.0) - 2026-08-23

### Added

- *(graph)* [**breaking**] add interstitial lanes

### Other

- *(readme)* update example graph to use "branch" label
- *(deps)* bump serde ([#8](https://github.com/kdheepak/renderdag/pull/8))
- *(deps)* bump actions/checkout from 6 to 7 ([#7](https://github.com/kdheepak/renderdag/pull/7))
- *(readme)* update description, add install and license sections

## [0.3.1](https://github.com/kdheepak/renderdag/compare/v0.3.0...v0.3.1) - 2026-04-08

### Other

- *(lib)* reformat code for improved readability and consistency
- *(graph)* reformat code for improved readability and consistency
- *(ci)* update mise-action to v4 in workflow
- *(lib)* format imports and update string push usage in tests
- *(graph)* group scratch data into structs for build and collapse steps
- add mise config, tasks, and rust toolchain for CI integration

## [0.3.0](https://github.com/kdheepak/renderdag/compare/v0.2.1...v0.3.0) - 2026-03-22

### Added

- *(graph)* [**breaking**] add render_terminal_lanes option to RenderConfig and usage
- *(graph)* add terminal lane rendering with config to graph output
- *(graph)* [**breaking**] add ParentAvailability and MissingParentState for parent tracking

### Fixed

- *(tests)* handle extra lines in render_with_suffix output

### Other

- *(lib)* remove terminal lane cap lines from test outputs
- add test_output_with_config for custom render config in tests
- *(graph)* replace collapse loop with compact_lanes_direct function
- *(lib)* add vertical line symbols to diagram strings
- *(cargo)* Update description
- *(graph)* inline push_route
- update test name

## [0.2.1](https://github.com/kdheepak/renderdag/compare/v0.2.0...v0.2.1) - 2026-02-11

### Other

- Add github workflows
- Initial commit
