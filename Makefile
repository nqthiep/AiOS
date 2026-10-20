# Kiểm tra theo docs/m0/README.md §6
.PHONY: check fmt vm-test rust-check py-check req-check

check: rust-check py-check req-check

rust-check:
	cargo fmt --all --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo test --workspace
	cargo test -p aios-policy --features spec-tests
	cargo test -p aios-confirm --features spec-tests

py-check:
	uv run --directory python/aiosd-cognition ruff check .
	uv run --directory python/aiosd-cognition pytest -q

req-check:
	python3 -I scripts/extract_requirements.py --check

fmt:
	cargo fmt --all
	uv run --directory python/aiosd-cognition ruff format .

vm-test:
	dev/vm-run.sh 'cargo test --workspace'
