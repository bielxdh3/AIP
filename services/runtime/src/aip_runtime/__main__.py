"""Command entry point for the managed AIP runtime."""

from __future__ import annotations

import argparse
import sys

from .protocol import MAX_MESSAGE_BYTES, encode_message, handle_line, health_document


def _serve_stdio() -> int:
    while raw_line := sys.stdin.buffer.readline(MAX_MESSAGE_BYTES + 2):
        if len(raw_line) > MAX_MESSAGE_BYTES:
            response, should_stop = handle_line(" " * (MAX_MESSAGE_BYTES + 1))
        else:
            try:
                line = raw_line.decode("utf-8", errors="strict").rstrip("\r\n")
            except UnicodeDecodeError:
                line = "{invalid-utf8"
            response, should_stop = handle_line(line)

        sys.stdout.write(encode_message(response) + "\n")
        sys.stdout.flush()
        if should_stop:
            return 0
    return 0


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
