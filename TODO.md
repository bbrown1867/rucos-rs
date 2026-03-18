# TODO

## Features

- [x] Implement multi-tasking
- [ ] Implement inter-task communication
- [ ] Implement synchronization primitives
- [ ] Implement time slicing for tasks with the same priority

## Ports

- [x] Create `cortex-m` port
- [ ] Create `risc-v` port

## Infrastructure

- [x] Add GitHub Actions for build, unit tests, and crate publishing
- [x] Remove `nightly` dependency in `cortex-m` crate: Using `naked_functions` feature for context switching
