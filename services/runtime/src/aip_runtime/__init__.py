"""AIP managed runtime boundary."""

from .protocol import PROTOCOL_VERSION, handle_line, health_document

__all__ = ["PROTOCOL_VERSION", "health_document", "handle_line"]
__version__ = "0.1.0"
