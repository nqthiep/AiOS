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

.PHONY: image

image:
	docker build --platform linux/arm64 -f image/Containerfile -t aios:m0 .

# Dependency licence check (spike S-16, docs/spikes/S-16-licenses.md).
# Rust: deny.toml. Python: every uv project tracked in git, dev group included.
# Allowed Python licence strings: the SPDX ids of deny.toml, the classifier
# name "MIT License", and SPDX "OR" expressions whose every choice is allowed.
# Per-package exceptions (reasons in the report): cffi (MIT-0),
# typing_extensions (PSF-2.0).
PY_LICENSE_ALLOW := MIT;MIT License;Apache-2.0;BSD-2-Clause;BSD-3-Clause;ISC;Unicode-3.0;Zlib;MPL-2.0;Apache-2.0 OR BSD-2-Clause;Apache-2.0 OR BSD-3-Clause
PY_LICENSE_EXCEPT := cffi typing_extensions
# dev/ holds VM-only spikes that are not shipped (dev/vm/hindsight pulls
# LGPL psycopg2-binary); their licenses are reviewed in the spike reports.
PY_PROJECTS := $(patsubst %/pyproject.toml,%,$(shell git ls-files '*pyproject.toml' ':!:dev/**'))

.PHONY: licenses

licenses:
	cargo deny check licenses
	@set -e; for p in $(PY_PROJECTS); do \
		echo "== $$p"; \
		uv sync --frozen --quiet --directory $$p; \
		uvx pip-licenses==5.5.5 --python $$p/.venv/bin/python --from=mixed --with-system \
			--allow-only="$(PY_LICENSE_ALLOW)" --ignore-packages $(PY_LICENSE_EXCEPT); \
	done
