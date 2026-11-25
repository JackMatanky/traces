# traces-bench-no-rustflags
paths: .github/workflows/bench.yml, .mise/tasks/bench/**, .mise/scripts/criterion_store.sh
forbid: /\bRUSTFLAGS\b|CARGO_ENCODED_RUSTFLAGS/
Do not set RUSTFLAGS in benchmark measurement runs. Custom flags change the build and break fair comparison.
