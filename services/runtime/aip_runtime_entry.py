"""PyInstaller entry point that preserves the aip_runtime package context."""

from aip_runtime.__main__ import main

if __name__ == "__main__":
    raise SystemExit(main())
