# Decision Experiments — Phase 0

Isolated, disposable experiments for blocking implementation choices. See [CONTEXT.md](../../Project_Phases/phase_0/CONTEXT.md).

**Rules:** Not imported by `core/` until explicitly promoted. Results feed [DECISIONS.md](../../Project_Phases/phase_0/DECISIONS.md).

## Experiments

| Directory | Decisions | Run |
|-----------|-----------|-----|
| `rust_sign_verify/` | DEC-001, DEC-004A | `cargo run` |
| `serialization_compare/` | DEC-003 | `cargo run` |
| `signing_pipeline/` | DEC-004B | `cargo run` |
| `python_interop/` | DEC-003, DEC-004B | `python3 verify_fixtures.py` |

## Requirements

Rust stable (see DEC-001). From each experiment directory:

```bash
cargo test
cargo run
```

Results are captured in `RESULTS.md` at this level after each run.
