from aiosd_cognition import cli


def test_cli_stub_returns_nonzero() -> None:
    assert cli.main() == 2
