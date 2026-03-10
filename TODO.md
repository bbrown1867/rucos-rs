# TODO

## Features

- [x] Implement multi-tasking
- [ ] Implement synchronization primitives
- [ ] Implement static memory pools with `Drop` trait
- [ ] Implement inter-task communication with `Send` and `Sync` traits
- [ ] Support time slicing if tasks with the same priority are ready

## Ports

- [x] Create a `cortex-m` port
- [ ] Create a `risc-v` port

## Infrastructure

- [x] Add GitHub Actions for build, unit tests, and crate publishing
- [x] Remove `nightly` dependency in `cortex-m` crate: Using `naked_functions` feature for context switching
