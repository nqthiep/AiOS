"""Entry point of the `aios.fs` MCP plugin (stdio transport).

The plugin host starts it as documented in `plugin.toml`:

    uv run --directory <plugin dir> python -I server.py
"""

from aios_fs.server import main

if __name__ == "__main__":
    main()
