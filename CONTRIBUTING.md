# Contributing

## Development

```bash
cargo fmt
cargo test
```

## Expectations

- Keep changes native Rust unless there is a hard external requirement
- Preserve output correctness before chasing smaller SVGs
- Add or update tests for every behavior change
- Keep CLI output stable and machine-readable where JSON is already supported

## Pull Requests

- explain the user-visible behavior change
- include verification notes
- include benchmark deltas when changing vectorization logic
