from __future__ import annotations

import argparse
import json
from pathlib import Path

from . import vectorize


def main() -> None:
    parser = argparse.ArgumentParser(prog="edgesvg")
    parser.add_argument("input")
    parser.add_argument("output")
    parser.add_argument("--method", default="hifi")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    result = vectorize(args.input, method=args.method)
    Path(args.output).write_text(result["svg"], encoding="utf-8")
    if args.json:
        print(json.dumps(result["report"], indent=2))
    else:
        print(args.output)


if __name__ == "__main__":
    main()
