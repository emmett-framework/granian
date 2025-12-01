#!/usr/bin/env bash

noir -v "benv=$BENV" \
    -c data:results/base.json \
    templates/main.md > README.md

noir -v "benv=$BENV" \
    -c data:results/vs.json \
    -c wsdata:results/vs_ws.json \
    templates/vs.md > vs.md

noir -v "benv=$BENV" -v pyvb=310 \
    -c data310:results/py310.json \
    -c data311:results/py311.json \
    -c data312:results/py312.json \
    -c data312:results/py312.json \
    -c data313:results/py313.json \
    -c data314:results/py314.json \
    templates/pyver.md > pyver.md

noir -v "benv=$BENV" \
    -c datal:results/loops.json \
    -c datat310:results/ti_py310.json \
    -c datat311:results/ti_py311.json \
    templates/asyncio.md > asyncio.md

noir -v "benv=$BENV" \
    -c data:results/concurrency.json \
    templates/concurrency.md > concurrency.md
