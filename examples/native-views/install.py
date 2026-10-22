"""Install the native views extension module over the stock Granian one.

The extension module built here is a drop-in replacement of the one shipped
with the Granian package: the Python side is entirely stock Granian, while the
native routes are provided by this crate.
"""

import pathlib
import shutil
import sys

import granian


def main() -> None:
    if len(sys.argv) < 2:
        print(f'usage: {sys.argv[0]} <path to the built extension module>')
        raise SystemExit(1)

    source = pathlib.Path(sys.argv[1]).resolve()
    if not source.is_file():
        print(f'{source} does not exist')
        raise SystemExit(1)

    package = pathlib.Path(granian.__path__[0])
    for existing in package.glob('_granian*.so'):
        existing.unlink()

    target = package / '_granian.so'
    shutil.copy(source, target)
    print(f'installed {target}')


if __name__ == '__main__':
    main()
