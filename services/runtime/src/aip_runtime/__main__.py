"""Command entry point for the managed AIP runtime."""

from __future__ import annotations

import argparse
import sys

from .protocol import encode_message, health_document
from .server import RuntimeServer


def _serve_stdio() -> int:
    return RuntimeServer(sys.stdout).serve(sys.stdin.buffer)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="aip-runtime")
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--health", action="store_true")
    mode.add_argument("--stdio", action="store_true")
    args = parser.parse_args(argv)

    if args.health:
        sys.stdout.write(encode_message(health_document()) + "\n")
        return 0
    return _serve_stdio()


if __name__ == "__main__":
    raise SystemExit(main())
