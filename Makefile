.DEFAULT_GOAL := all
black = black granian tests
ruff = ruff granian tests

.PHONY: build-dev
build-dev:
	@rm -f granian/*.so
	maturin develop --extras test

.PHONY: format
format:
	$(black)
	$(ruff) --fix --exit-zero
	cargo fmt

.PHONY: lint-python
lint-python:
	$(ruff)
	$(black) --check --diff

.PHONY: lint-rust
lint-rust:
	cargo fmt --version
	cargo fmt --all -- --check
	cargo clippy --version
	cargo clippy --tests -- \
		-D warnings \
		-W clippy::pedantic \
		-W clippy::dbg_macro \
		-W clippy::print_stdout \
		-A clippy::cast-possible-truncation \
		-A clippy::cast-possible-wrap \
		-A clippy::cast-precision-loss \
		-A clippy::cast-sign-loss \
		-A clippy::declare-interior-mutable-const \
		-A clippy::float-cmp \
		-A clippy::fn-params-excessive-bools \
		-A clippy::if-not-else \
		-A clippy::inline-always \
		-A clippy::manual-let-else \
		-A clippy::match-bool \
		-A clippy::match-same-arms \
		-A clippy::missing-errors-doc \
		-A clippy::missing-panics-doc \
		-A clippy::module-name-repetitions \
		-A clippy::must-use-candidate \
		-A clippy::needless-pass-by-value \
		-A clippy::similar-names \
		-A clippy::single-match-else \
		-A clippy::struct-excessive-bools \
		-A clippy::too-many-arguments \
		-A clippy::too-many-lines \
		-A clippy::type-complexity \
		-A clippy::unnecessary-wraps \
		-A clippy::unused-self \
		-A clippy::used-underscore-binding \
		-A clippy::wrong-self-convention

.PHONY: lint
lint: lint-python lint-rust

.PHONY: test
test:
	pytest -v test

.PHONY: all
all: format build-dev lint test
