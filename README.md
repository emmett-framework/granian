# Granian

A Rust HTTP server for Python applications.

## Rationale

Main reasons behind Granian design are:

- Have a single, correct HTTP implementation, supporting versions 1, 2 (and eventually 3)
- Provide a single package for several platforms (avoiding the usual Gunicorn + uvicorn + http-tools dependency composition)
- Provide stable performance comparable with existing alternatives

## Project status

Granian is currently in early-stage development.

## License

Granian is released under the BSD License.
