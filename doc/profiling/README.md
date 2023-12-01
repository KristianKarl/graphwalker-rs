# Profiling

## Using cargo-flamegraph

```bash
cargo install flamegraph

echo -1 | sudo tee /proc/sys/kernel/perf_event_paranoid

# Run 10 seconds; stop by ctrl+c
export CARGO_PROFILE_RELEASE_DEBUG=true; cargo flamegraph --bin graphwalker -- offline resources/models/SuperLarge.json
cp flamegraph.svg > doc/profiling/flamegraph_$(git rev-parse --short HEAD).svg
```

### 2023-11-30 [flamegraph_7a3d6d5.svg](flamegraph_7a3d6d5.svg)

This version of graphealker prints around 20.000 lines, wheras the java version prints 120.000 lines.
A lot of allocation on the heap seems to slow it down due to cloning I guess.
