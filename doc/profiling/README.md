# Profiling

## Using cargo-flamegraph

```bash
cargo install flamegraph

echo -1 | sudo tee /proc/sys/kernel/perf_event_paranoid

# install timeout to get comparable runtimes
export CARGO_PROFILE_RELEASE_DEBUG=true; timeout 10s cargo flamegraph --bin graphwalker -- offline resources/models/SuperLarge.json
cp flamegraph.svg  doc/profiling/flamegraph_$(git rev-parse --short HEAD).svg
```

### 2023-11-30 [flamegraph_7a3d6d5.svg](flamegraph_7a3d6d5.svg)

This version of graphealker prints around 20.000 lines, wheras the java version prints 120.000 lines.
A lot of allocation on the heap seems to slow it down due to cloning I guess.

### 2023-12-01 [flamegraph_41418e4.svg](flamegraph_41418e4.svg)

Improved printouts to ~300.000 lines.

### 2023-12-01 [flamegraph_e934688.svg](flamegraph_e934688.svg)

Improved printouts to ~360.000 lines.

### 2023-12-01 [flamegraph_4dc1ab1.svg](flamegraph_4dc1ab1.svg)

Improved printouts to ~450.000 lines.