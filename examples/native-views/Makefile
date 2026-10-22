# The extension module must be built against the same Python interpreter the
# Granian package is installed for.
#
# In a working copy, the dependencies are usually installed in a local virtual
# environment (the `.venv` directory created by `uv`, which is not tracked by
# the repository):
#
#     make build PYTHON=.venv/bin/python
#
# When the environment is already the active one, the default is enough:
#
#     make build
PYTHON ?= python3

.PHONY: build
build:
	PYO3_PYTHON=$(PYTHON) cargo build --release

.PHONY: install
install: build
	$(PYTHON) install.py target/release/libgranian_native_views.so

.PHONY: clean
clean:
	cargo clean
