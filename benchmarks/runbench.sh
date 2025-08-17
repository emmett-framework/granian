#!/usr/bin/env bash

rm -rf ./benchmarks/.envs
rm -rf ./target
mkdir -p ./benchmarks/.envs

uv venv -p 3.10 ./benchmarks/.envs/.venv310
uv venv -p 3.11 ./benchmarks/.envs/.venv311
uv venv -p 3.12 ./benchmarks/.envs/.venv312
uv venv -p 3.13 ./benchmarks/.envs/.venv313
uv venv -p 3.14 ./benchmarks/.envs/.venv314

uv sync --group build

uv run maturin build --release --features jemalloc --interpreter ./benchmarks/.envs/.venv310/bin/python
uv run maturin build --release --features jemalloc --interpreter ./benchmarks/.envs/.venv311/bin/python
uv run maturin build --release --features jemalloc --interpreter ./benchmarks/.envs/.venv312/bin/python
uv run maturin build --release --features jemalloc --interpreter ./benchmarks/.envs/.venv313/bin/python
uv run maturin build --release --features jemalloc --interpreter ./benchmarks/.envs/.venv314/bin/python

VIRTUAL_ENV=$(pwd)/benchmarks/.envs/.venv310 uv pip install $(ls target/wheels/granian-*-cp310-*.whl)
VIRTUAL_ENV=$(pwd)/benchmarks/.envs/.venv311 uv pip install $(ls target/wheels/granian-*-cp311-*.whl)
VIRTUAL_ENV=$(pwd)/benchmarks/.envs/.venv312 uv pip install $(ls target/wheels/granian-*-cp312-*.whl)
VIRTUAL_ENV=$(pwd)/benchmarks/.envs/.venv313 uv pip install $(ls target/wheels/granian-*-cp313-*.whl)
VIRTUAL_ENV=$(pwd)/benchmarks/.envs/.venv314 uv pip install $(ls target/wheels/granian-*-cp314-*.whl)

VIRTUAL_ENV=$(pwd)/benchmarks/.envs/.venv310 uv pip install -r ./benchmarks/envs/common.txt
VIRTUAL_ENV=$(pwd)/benchmarks/.envs/.venv311 uv pip install -r ./benchmarks/envs/common.txt
VIRTUAL_ENV=$(pwd)/benchmarks/.envs/.venv312 uv pip install -r ./benchmarks/envs/common.txt
VIRTUAL_ENV=$(pwd)/benchmarks/.envs/.venv313 uv pip install -r ./benchmarks/envs/common.txt
VIRTUAL_ENV=$(pwd)/benchmarks/.envs/.venv314 uv pip install -r ./benchmarks/envs/common.txt
VIRTUAL_ENV=$(pwd)/benchmarks/.envs/.venv313 uv pip install -r ./benchmarks/envs/asgi.txt
VIRTUAL_ENV=$(pwd)/benchmarks/.envs/.venv313 uv pip install -r ./benchmarks/envs/wsgi.txt

cd ./benchmarks

# base bench
BENCHMARK_EXC_PREFIX=$(pwd)/.envs/.venv313/bin ./.envs/.venv313/bin/python benchmarks.py
mv ./results/data.json ./results/base.json

# vs bench
BENCHMARK_EXC_PREFIX=$(pwd)/.envs/.venv313/bin ./.envs/.venv313/bin/python benchmarks.py vs
mv ./results/data.json ./results/vs.json

# ws
BENCHMARK_EXC_PREFIX=$(pwd)/.envs/.venv313/bin ./.envs/.venv313/bin/python benchmarks.py vs_ws
mv ./results/data.json ./results/vs_ws.json

# pyver
BENCHMARK_EXC_PREFIX=$(pwd)/.envs/.venv310/bin ./.envs/.venv310/bin/python benchmarks.py interfaces
mv ./results/data.json ./results/py310.json
BENCHMARK_EXC_PREFIX=$(pwd)/.envs/.venv311/bin ./.envs/.venv311/bin/python benchmarks.py interfaces
mv ./results/data.json ./results/py311.json
BENCHMARK_EXC_PREFIX=$(pwd)/.envs/.venv312/bin ./.envs/.venv312/bin/python benchmarks.py interfaces
mv ./results/data.json ./results/py312.json
BENCHMARK_EXC_PREFIX=$(pwd)/.envs/.venv313/bin ./.envs/.venv313/bin/python benchmarks.py interfaces
mv ./results/data.json ./results/py313.json
BENCHMARK_EXC_PREFIX=$(pwd)/.envs/.venv314/bin ./.envs/.venv314/bin/python benchmarks.py interfaces
mv ./results/data.json ./results/py314.json

# loops
BENCHMARK_EXC_PREFIX=$(pwd)/.envs/.venv313/bin ./.envs/.venv313/bin/python benchmarks.py loops
mv ./results/data.json ./results/loops.json

# task-impl
BENCHMARK_EXC_PREFIX=$(pwd)/.envs/.venv310/bin ./.envs/.venv310/bin/python benchmarks.py task_impl
mv ./results/data.json ./results/ti_py310.json
BENCHMARK_EXC_PREFIX=$(pwd)/.envs/.venv311/bin ./.envs/.venv311/bin/python benchmarks.py task_impl
mv ./results/data.json ./results/ti_py311.json

# concurrency
BENCHMARK_EXC_PREFIX=$(pwd)/.envs/.venv313/bin ./.envs/.venv313/bin/python benchmarks.py concurrency
mv ./results/data.json ./results/concurrency.json
