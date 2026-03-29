---
title: Batch Conversion Guide
description: Convert entire folders of images to SVG in parallel using the CLI or any SDK.
---

EdgeSVG is designed for high-throughput batch processing. All SDKs support concurrent conversion with automatic quality reporting.

## CLI — quickest path

```bash
# Convert all PNGs in a folder, output to svg/
edgesvg batch "icons/*.png" icons/svg/ --json

# All sub-folders, 8 threads
edgesvg batch "assets/**/*.png" out/ --jobs 8 --method auto

# Only continue on errors (skip failed files)
edgesvg batch "*.png" out/ --continue-on-error
```

After the run, `out/report.json` is written with per-file metrics:

```json
[
  { "file": "logo.png", "grade": "A+", "ssim": 0.9990, "elapsed_ms": 120 },
  { "file": "icon-home.png", "grade": "A", "ssim": 0.9942, "elapsed_ms": 85 }
]
```

## Python — parallel with `concurrent.futures`

```python
import edgesvg
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
import json

def convert_one(png_path: Path, out_dir: Path) -> dict:
    out = out_dir / png_path.with_suffix(".svg").name
    result = edgesvg.vectorize(str(png_path), output_path=str(out), method="auto")
    return {
        "file": png_path.name,
        "grade": result["report"]["grade"],
        "ssim": result["report"]["metrics"]["ssim"],
    }

def batch_convert(input_dir: str, output_dir: str, workers: int = 8):
    input_path = Path(input_dir)
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)

    pngs = list(input_path.rglob("*.png"))
    results = []

    with ThreadPoolExecutor(max_workers=workers) as pool:
        futures = {pool.submit(convert_one, p, output_path): p for p in pngs}
        for future in as_completed(futures):
            try:
                results.append(future.result())
            except Exception as e:
                results.append({"file": str(futures[future]), "error": str(e)})

    with open(output_path / "report.json", "w") as f:
        json.dump(results, f, indent=2)

    passed = [r for r in results if "ssim" in r and r["ssim"] >= 0.99]
    print(f"Converted {len(passed)}/{len(pngs)} files above SSIM 0.99")
    return results
```

## Node.js — parallel with `Promise.all`

```typescript
import { vectorize } from 'edgesvg-node';
import { glob } from 'glob';
import { writeFileSync } from 'fs';
import path from 'path';

async function batchConvert(inputDir: string, outputDir: string) {
  const files = await glob(`${inputDir}/**/*.png`);

  // Chunk into batches to control memory
  const CONCURRENCY = 8;
  const results: object[] = [];

  for (let i = 0; i < files.length; i += CONCURRENCY) {
    const chunk = files.slice(i, i + CONCURRENCY);
    const batch = await Promise.all(
      chunk.map(async (file) => {
        const name = path.basename(file, '.png') + '.svg';
        const output = path.join(outputDir, name);
        const result = await vectorize(file, { outputPath: output, method: 'auto' });
        return {
          file: path.basename(file),
          grade: result.report.grade,
          ssim: result.report.metrics.ssim,
        };
      })
    );
    results.push(...batch);
    console.log(`Progress: ${Math.min(i + CONCURRENCY, files.length)}/${files.length}`);
  }

  writeFileSync(`${outputDir}/report.json`, JSON.stringify(results, null, 2));
  return results;
}
```

## Rust — full parallelism with rayon

```rust
use edgesvg::{vectorize_file, VectorizationConfig, VectorizeMethod};
use rayon::prelude::*;
use std::path::{Path, PathBuf};

fn batch_convert(inputs: Vec<PathBuf>, out_dir: &Path) -> Vec<serde_json::Value> {
    let config = VectorizationConfig {
        method: VectorizeMethod::Auto,
        ..Default::default()
    };

    inputs
        .par_iter()
        .map(|input| {
            let stem = input.file_stem().unwrap().to_str().unwrap();
            let output = out_dir.join(format!("{}.svg", stem));

            match vectorize_file(input, Some(&output), &config) {
                Ok(resp) => serde_json::json!({
                    "file": input.file_name().unwrap().to_str(),
                    "grade": resp.report.grade,
                    "ssim": resp.report.metrics.ssim,
                }),
                Err(e) => serde_json::json!({
                    "file": input.file_name().unwrap().to_str(),
                    "error": e.to_string(),
                }),
            }
        })
        .collect()
}
```

## CI Quality Gating

After a batch run, fail CI if any file is below the quality threshold:

```bash
#!/bin/bash
# scripts/quality-check.sh

edgesvg batch "src/assets/*.png" dist/svg/ --json
FAIL=$(jq '[.[] | select(.ssim < 0.99)] | length' dist/svg/report.json)

if [ "$FAIL" -gt 0 ]; then
  echo "❌ $FAIL files failed quality check (SSIM < 0.99)"
  jq '.[] | select(.ssim < 0.99) | {file, ssim, grade}' dist/svg/report.json
  exit 1
fi

echo "✅ All files passed quality check"
```

```yaml
# .github/workflows/build.yml
- name: Quality check SVG assets
  run: bash scripts/quality-check.sh
```

## Performance Tips

- **Use `--jobs`** to match your CPU count for maximum throughput
- **Use `smart` mode** for guaranteed quality; use `auto` for speed
- **Process incrementally** — compare timestamps to skip unchanged files
- **Stream large sets** — the Python SDK processes files lazily; don't load all at once

### Expected throughput

| Hardware | Images/sec | Notes |
|----------|------------|-------|
| MacBook M3 (8 cores) | 40–80/s | Depends on image size |
| GitHub Actions (2 cores) | 15–25/s | Default runner |
| AWS c7g.4xlarge (16 cores) | 120–200/s | ARM64 |
