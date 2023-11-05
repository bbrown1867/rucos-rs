# TODO

## Features

- [x] Implement multi-tasking
- [ ] Implement synchronization primitives: Semaphore, mutex, event flags
- [ ] Implement inter-task communication: See _Hubris_ for use of `Send` and `Sync` traits
- [ ] Implement static memory pools: Use language features for memory safety (e.g. `Drop` trait)
- [ ] Support time slicing if multiple tasks with the same priority are ready

## Ports

- [x] Create a `cortex-m` port
- [ ] Create a `risc-v` port

## Infrastructure

- [x] Add GitHub Actions for build, unit tests, and crate publishing
- [ ] Remove `nightly` dependency in `cortex-m` crate: Using `naked_functions` feature for context switching
