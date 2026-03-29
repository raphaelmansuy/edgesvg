---
title: CI/CD Integration
description: Add EdgeSVG quality gates to GitHub Actions, GitLab CI, and other pipelines.
---

EdgeSVG integrates cleanly into CI/CD pipelines with exit codes, JSON output, and configurable quality thresholds.

## GitHub Actions

### Basic quality check

```yaml
# .github/workflows/svg-quality.yml
name: SVG Quality

on:
  push:
    paths:
      - 'assets/images/**'

jobs:
  vectorize:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install EdgeSVG
        run: cargo install edgesvg

      - name: Convert and check quality
        run: |
          edgesvg batch "assets/images/*.png" dist/svg/ --json --method auto
          jq -e '[.[] | select(.ssim < 0.99)] | length == 0' dist/svg/report.json \
            || (echo "Quality gates failed" && exit 1)

      - name: Upload SVG artifacts
        uses: actions/upload-artifact@v4
        with:
          name: svg-assets
          path: dist/svg/
```

### Python SDK in Actions

```yaml
name: Asset Pipeline

on: [push, pull_request]

jobs:
  assets:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-python@v5
        with:
          python-version: '3.12'

      - name: Install EdgeSVG
        run: pip install edgesvg

      - name: Convert icons
        run: python scripts/convert_icons.py

      - name: Fail on quality issues
        run: |
          python - <<'EOF'
          import json, sys
          results = json.load(open("dist/report.json"))
          failed = [r for r in results if r.get("ssim", 0) < 0.989]
          if failed:
              print(f"FAILED: {len(failed)} files below threshold")
              for f in failed:
                  print(f"  {f['file']}: SSIM={f.get('ssim', 'N/A')}")
              sys.exit(1)
          print(f"PASSED: all {len(results)} files above threshold")
          EOF
```

### Caching Rust builds

```yaml
- uses: Swatinem/rust-cache@v2
  with:
    cache-on-failure: true

- name: Install EdgeSVG
  run: cargo install edgesvg --locked
```

---

## GitLab CI

```yaml
# .gitlab-ci.yml
stages:
  - vectorize

svg-quality:
  stage: vectorize
  image: rust:latest
  before_script:
    - cargo install edgesvg
  script:
    - edgesvg batch "assets/**/*.png" dist/svg/ --json --continue-on-error
    - |
      FAIL=$(jq '[.[] | select(.ssim < 0.99)] | length' dist/svg/report.json)
      test "$FAIL" -eq 0 || (echo "$FAIL files failed quality check" && exit 1)
  artifacts:
    paths:
      - dist/svg/
    reports:
      junit: dist/svg/report.json
  only:
    changes:
      - assets/**
```

---

## Docker

Build a container with EdgeSVG for portable CI:

```dockerfile
FROM rust:1.82-slim AS builder
RUN cargo install edgesvg

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/edgesvg /usr/local/bin/edgesvg
ENTRYPOINT ["edgesvg"]
```

```bash
docker build -t edgesvg:latest -f Dockerfile.ci .

# Run in CI
docker run --rm -v $PWD:/workspace edgesvg:latest \
  batch "/workspace/assets/*.png" /workspace/dist/svg/ --json
```

---

## Quality Threshold Reference

Suggested thresholds for different asset types:

| Asset Class | Min SSIM | Min EdgeF1 | Grade Target |
|-------------|----------|------------|--------------|
| Brand logos | 0.998 | 0.985 | A+ |
| App icons | 0.995 | 0.980 | A |
| UI elements | 0.990 | 0.970 | A |
| Illustrations | 0.985 | 0.960 | A |
| Marketing | 0.980 | 0.950 | B+ |

---

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | All conversions succeeded and quality targets met |
| `1` | Conversion error |
| `2` | Quality threshold not met (smart mode) |
| `3` | Invalid arguments |
| `4` | I/O error |

Use conditional logic in shell:

```bash
edgesvg convert input.png output.svg --method smart --target-ssim 0.998
if [ $? -eq 2 ]; then
  echo "Quality target not met — check the image source"
fi
```

---

## Pre-commit Hook

Install as a pre-commit hook to check SVG quality before committing:

```bash
# .git/hooks/pre-commit
#!/bin/bash

# Find new/modified PNGs staged for commit
STAGED_PNGS=$(git diff --cached --name-only --diff-filter=ACM | grep '\.png$')

if [ -z "$STAGED_PNGS" ]; then
  exit 0
fi

echo "EdgeSVG: checking $STAGED_PNGS..."

for PNG in $STAGED_PNGS; do
  RESULT=$(edgesvg convert "$PNG" --method auto --json 2>/dev/null)
  GRADE=$(echo "$RESULT" | jq -r '.report.grade // "F"')

  if [[ "$GRADE" == "F" || "$GRADE" == "D" ]]; then
    echo "❌ $PNG: grade $GRADE — SVG quality too low"
    exit 1
  fi
done

echo "✅ All PNG assets pass quality check"
```

```bash
chmod +x .git/hooks/pre-commit
```

---

## Publishing SDK Releases

EdgeSVG ships four release workflows triggered automatically when a semver tag (e.g. `v0.3.0`) is pushed. Each workflow publishes to a different registry.

### Prerequisites — GitHub Environments & Secrets

Create three **Environments** in your GitHub repo (`Settings → Environments`):

| Environment | Required secret | Registry |
|---|---|---|
| `crates-io` | `CARGO_REGISTRY_TOKEN` | [crates.io](https://crates.io) |
| `pypi` | `PYPI_API_TOKEN` | [PyPI](https://pypi.org) |
| `npm` | `NPM_TOKEN` | [npmjs.com](https://www.npmjs.com) |

Environments add an **optional approval gate** before secrets are exposed — strongly recommended for production releases.

#### Get each token

**Rust / crates.io**
```bash
# Log in, then copy the token shown
cargo login
# Or: crates.io → Account Settings → API Tokens → New Token (scope: publish-new, publish-update)
```

**Python / PyPI**
```
pypi.org → Account Settings → API tokens → Add API token
Scope: Entire account (or restrict to the edgesvg project once it exists)
Name:  GITHUB_ACTIONS_EDGESVG
→ Copy token starting with pypi-…
```

**Node / npm**
```
npmjs.com → Profile → Access Tokens → Generate New Token → Automation
→ Copy token starting with npm_…
```

Add each token as a secret inside its corresponding GitHub Environment, **not** as a global repo secret — this prevents accidental exposure in other workflows.

---

### Release Workflow: Rust Crate

**File:** `.github/workflows/release-rust.yml`

```yaml
name: Release Rust

on:
  push:
    tags: ["v[0-9]+.[0-9]+.[0-9]+"]
  workflow_dispatch:

jobs:
  publish:
    runs-on: ubuntu-latest
    environment: crates-io          # ← requires CARGO_REGISTRY_TOKEN
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo publish --locked --token "${{ secrets.CARGO_REGISTRY_TOKEN }}"
```

**How to trigger:**
```bash
git tag v0.3.0
git push origin v0.3.0
```

---

### Release Workflow: Python Wheels

**File:** `.github/workflows/release-python.yml`

Builds wheels for Linux (`manylinux`), macOS, and Windows × Python 3.9–3.13 in parallel, then uploads to PyPI.

```yaml
name: Release Python

on:
  push:
    tags: ["v[0-9]+.[0-9]+.[0-9]+"]
  workflow_dispatch:

jobs:
  build:
    name: Build wheels
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-14, windows-latest]
        py: ["3.9", "3.10", "3.11", "3.12", "3.13"]
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: ${{ matrix.py }}
      - uses: dtolnay/rust-toolchain@stable
      - uses: PyO3/maturin-action@v1
        with:
          command: build
          working-directory: sdks/python
          manylinux: auto
          args: --release --out dist
      - uses: actions/upload-artifact@v4
        with:
          name: python-dist-${{ matrix.os }}-${{ matrix.py }}
          path: sdks/python/dist/*

  publish:
    needs: build
    runs-on: ubuntu-latest
    environment: pypi               # ← requires PYPI_API_TOKEN
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: "3.12"
      - uses: actions/download-artifact@v4
        with:
          path: dist
          merge-multiple: true
      - run: python -m pip install twine
      - run: python -m twine upload dist/*
        env:
          TWINE_USERNAME: __token__
          TWINE_PASSWORD: ${{ secrets.PYPI_API_TOKEN }}
```

**Dry-run before real publish:**
```bash
python -m twine upload --repository testpypi dist/*
# Set TWINE_PASSWORD to your TestPyPI token
```

---

### Release Workflow: Node Native Add-on

**File:** `.github/workflows/release-node.yml`

Compiles platform-specific `.node` binaries for 5 targets, uploads them as artifacts, then assembles the npm package and publishes.

Key targets:

| Target | Platform |
|---|---|
| `x86_64-unknown-linux-gnu` | Linux x64 |
| `aarch64-unknown-linux-gnu` | Linux ARM64 |
| `x86_64-apple-darwin` | macOS Intel |
| `aarch64-apple-darwin` | macOS Apple Silicon |
| `x86_64-pc-windows-msvc` | Windows x64 |

```yaml
# publish job (after build-native)
  publish:
    needs: build-native
    runs-on: ubuntu-latest
    environment: npm                # ← requires NPM_TOKEN
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: "20"
          registry-url: "https://registry.npmjs.org"
      - uses: actions/download-artifact@v4
        with:
          path: sdks/node/npm
          merge-multiple: true
      - name: Publish to npm
        working-directory: sdks/node
        run: npm publish --access public
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
```

---

### Release Workflow: WASM Package

**File:** `.github/workflows/release-wasm.yml`

Builds the WASM bundle with `wasm-pack` and uploads it as a GitHub Actions artifact. To publish to npm as `@edgesvg/wasm`, add a publish step:

```yaml
  publish:
    needs: build
    runs-on: ubuntu-latest
    environment: npm
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: "20"
          registry-url: "https://registry.npmjs.org"
      - uses: actions/download-artifact@v4
        with:
          name: edgesvg-wasm-pkg
          path: crates/edgesvg-wasm/pkg
      - name: Publish WASM to npm
        working-directory: crates/edgesvg-wasm/pkg
        run: npm publish --access public
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
```

---

### Release Checklist

Use this checklist every time you cut a release:

```
[ ] Update version in Cargo.toml (workspace root)
[ ] Update CHANGELOG.md with release notes
[ ] Run: cargo test --all && cargo clippy --all -- -D warnings
[ ] Commit: git commit -am "chore: release vX.Y.Z"
[ ] Tag:    git tag vX.Y.Z && git push origin vX.Y.Z
[ ] Monitor GitHub Actions — all 4 release workflows must go green
[ ] Verify on crates.io: https://crates.io/crates/edgesvg
[ ] Verify on PyPI:      https://pypi.org/project/edgesvg/
[ ] Verify on npm:       https://www.npmjs.com/package/@edgesvg/node
[ ] Test install: pip install edgesvg==X.Y.Z && cargo add edgesvg@X.Y.Z
```

---

### Site & Demo Deployment

The documentation site and demo app have their own workflow:

**File:** `.github/workflows/deploy-site.yml`

- **Trigger:** push to `main` touching `site/**`
- **Deploys to:** GitHub Pages at `https://raphaelmansuy.github.io/edgesvg/`
- **Required repo setting:** `Settings → Pages → Source: GitHub Actions`

No secrets needed — the workflow uses the built-in `GITHUB_TOKEN` with `pages: write` + `id-token: write` permissions.
