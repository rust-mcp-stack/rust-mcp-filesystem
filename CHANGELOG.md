# Changelog

## [0.2.3](https://github.com/rust-mcp-stack/rust-mcp-filesystem/compare/v0.2.2...v0.2.3) (2025-08-30)


### 🐛 Bug Fixes

* Issue-37 panic in search files content tool ([#38](https://github.com/rust-mcp-stack/rust-mcp-filesystem/issues/38)) ([1f7b985](https://github.com/rust-mcp-stack/rust-mcp-filesystem/commit/1f7b985ffc5bf6b6c00225c6755f1ae068fad1d5))
* Update rust-mcp-sdk dependency to the latest ([#39](https://github.com/rust-mcp-stack/rust-mcp-filesystem/issues/39)) ([9174be8](https://github.com/rust-mcp-stack/rust-mcp-filesystem/commit/9174be8c9286eb5245a4e0537e803dfff51a4cee))

## [0.2.2](https://github.com/rust-mcp-stack/rust-mcp-filesystem/compare/v0.2.1...v0.2.2) (2025-07-05)


### 🐛 Bug Fixes

* Homebrew build issue ([da85e41](https://github.com/rust-mcp-stack/rust-mcp-filesystem/commit/da85e4122ca67219d80d5b2946004bbc7986cef9))

## [0.2.1](https://github.com/rust-mcp-stack/rust-mcp-filesystem/compare/v0.2.0...v0.2.1) (2025-07-05)


### 🚀 Features

* Enhance directory_tree response with metadata ([#30](https://github.com/rust-mcp-stack/rust-mcp-filesystem/issues/30)) ([41e7424](https://github.com/rust-mcp-stack/rust-mcp-filesystem/commit/41e742401fdf0d09f74084d3ead6082bd7e51384))

## [0.2.0](https://github.com/rust-mcp-stack/rust-mcp-filesystem/compare/v0.1.10...v0.2.0) (2025-07-05)


### ⚠ BREAKING CHANGES

* upgrade to latest MCP protocol (2025-06-18) ([#29](https://github.com/rust-mcp-stack/rust-mcp-filesystem/issues/29))

### 🚀 Features

* Add list_directory_with_sizes MCP Tool for Directory Listing with File Sizes ([#27](https://github.com/rust-mcp-stack/rust-mcp-filesystem/issues/27)) ([15121c8](https://github.com/rust-mcp-stack/rust-mcp-filesystem/commit/15121c8d1605366ea5185f6a9e2ffd7036693d13))
* Upgrade to latest MCP protocol (2025-06-18) ([#29](https://github.com/rust-mcp-stack/rust-mcp-filesystem/issues/29)) ([cd6af1b](https://github.com/rust-mcp-stack/rust-mcp-filesystem/commit/cd6af1bfc14dab4b2ba68b014be860c8e9668667))


### 🐛 Bug Fixes

* Directory tree tool result ([#26](https://github.com/rust-mcp-stack/rust-mcp-filesystem/issues/26)) ([01f956e](https://github.com/rust-mcp-stack/rust-mcp-filesystem/commit/01f956efdde5fdd0e5fd14f30e4ebdac53d728f7))

## [0.1.10](https://github.com/rust-mcp-stack/rust-mcp-filesystem/compare/v0.1.9...v0.1.10) (2025-06-18)


### 🚀 Features

* Implement a new mcp tool for searching files content ([#23](https://github.com/rust-mcp-stack/rust-mcp-filesystem/issues/23)) ([950149a](https://github.com/rust-mcp-stack/rust-mcp-filesystem/commit/950149aa30542c8ffcba040de614861eda4b68da))

## [0.1.9](https://github.com/rust-mcp-stack/rust-mcp-filesystem/compare/v0.1.8...v0.1.9) (2025-05-29)


### 🐛 Bug Fixes

* File edit operation by adding bounds check ([#20](https://github.com/rust-mcp-stack/rust-mcp-filesystem/issues/20)) ([4fedd50](https://github.com/rust-mcp-stack/rust-mcp-filesystem/commit/4fedd5090e3204aee8f9dff9442953b8d2993616))

## [0.1.8](https://github.com/rust-mcp-stack/rust-mcp-filesystem/compare/v0.1.7...v0.1.8) (2025-05-25)


### 🐛 Bug Fixes

* Support clients with older versions of mcp protocol ([#17](https://github.com/rust-mcp-stack/rust-mcp-filesystem/issues/17)) ([4c14bde](https://github.com/rust-mcp-stack/rust-mcp-filesystem/commit/4c14bde9f9233535cdf0cb17127ed15a24d2650a))

## [0.1.7](https://github.com/rust-mcp-stack/rust-mcp-filesystem/compare/v0.1.6...v0.1.7) (2025-05-25)


### 🚀 Features

* Update mcp-sdk dependency for smaller binary size ([3db8038](https://github.com/rust-mcp-stack/rust-mcp-filesystem/commit/3db80384a9d7c975229cceb5a78e0c0e2cb6f2ef))

## [0.1.6](https://github.com/rust-mcp-stack/rust-mcp-filesystem/compare/v0.1.5...v0.1.6) (2025-05-25)


### 🚀 Features

* Upgrade to latest MCP schema version ([f950fcf](https://github.com/rust-mcp-stack/rust-mcp-filesystem/commit/f950fcf086da51115426796e474ba1d6180e3b01))


### 🐛 Bug Fixes

* Issue 12 edit_file tool panics ([#14](https://github.com/rust-mcp-stack/rust-mcp-filesystem/issues/14)) ([25da5a6](https://github.com/rust-mcp-stack/rust-mcp-filesystem/commit/25da5a674a0455d9e752da65b5fcb94053aa40b1))

## [0.1.5](https://github.com/rust-mcp-stack/rust-mcp-filesystem/compare/v0.1.4...v0.1.5) (2025-05-01)


### 🚀 Features

* Improve tools descriptions ([1f9fa19](https://github.com/rust-mcp-stack/rust-mcp-filesystem/commit/1f9fa193bc09e45179fa1c42e00b1e67c979e134))
* Improve tools descriptions ([f3129e7](https://github.com/rust-mcp-stack/rust-mcp-filesystem/commit/f3129e7188986899f099e9bf211fb1b960081645))

## [0.1.4](https://github.com/rust-mcp-stack/rust-mcp-filesystem/compare/v0.1.3...v0.1.4) (2025-04-28)


### 🚀 Features

* Update rust-mcp-sdk and outdated dependencies ([cf62128](https://github.com/rust-mcp-stack/rust-mcp-filesystem/commit/cf62128d2845566fc900aaee62f5932f6bda0e72))
* Update rust-mcp-sdk to latest version ([c59b685](https://github.com/rust-mcp-stack/rust-mcp-filesystem/commit/c59b6854f5df6fd2d98232eff72e9a635cb08bd5))

## [0.1.3](https://github.com/rust-mcp-stack/rust-mcp-filesystem/compare/v0.1.2...v0.1.3) (2025-04-18)


### 🐛 Bug Fixes

* Update cargo dist ([4ded5ca](https://github.com/rust-mcp-stack/rust-mcp-filesystem/commit/4ded5cae9fc292dfea821f82aeaea5eea2c069ca))

## [0.1.2](https://github.com/rust-mcp-stack/rust-mcp-filesystem/compare/v0.1.1...v0.1.2) (2025-04-18)


### 🐛 Bug Fixes

* Cargo dist update ([8ef4393](https://github.com/rust-mcp-stack/rust-mcp-filesystem/commit/8ef43935a5fb92df33da36e12812de004e337a57))

## [0.1.1](https://github.com/rust-mcp-stack/rust-mcp-filesystem/compare/v0.1.0...v0.1.1) (2025-04-18)


### ⚙️ Miscellaneous Chores

* Release 0.1.1 ([d9c0fb6](https://github.com/rust-mcp-stack/rust-mcp-filesystem/commit/d9c0fb608bf8fe799ca0b6b853c8299226362531))

## [0.1.0](https://github.com/rust-mcp-stack/rust-mcp-filesystem/compare/v0.1.0...v0.1.0) (2025-04-18)


### ⚙️ Miscellaneous Chores

* Release 0.1.0 ([042f817](https://github.com/rust-mcp-stack/rust-mcp-filesystem/commit/042f817ab05129706e532991ef14fc7a4d23bda6))
