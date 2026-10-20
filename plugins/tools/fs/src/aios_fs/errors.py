"""Errors raised by the `aios.fs` plugin."""


class FsError(Exception):
    """A failure the caller can act on (surfaced to the model as an MCP tool error)."""
