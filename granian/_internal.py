import os
import re
import sys
import traceback
from pathlib import Path
from types import ModuleType
from typing import Callable, List, Optional

from ._imports import dotenv


def get_import_components(path: str) -> List[Optional[str]]:
    return (re.split(r':(?![\\/])', path, maxsplit=1) + [None])[:2]


def prepare_import(path: str) -> str:
    path = os.path.realpath(path)

    fname, ext = os.path.splitext(path)
    if ext == '.py':
        path = fname
    if os.path.basename(path) == '__init__':
        path = os.path.dirname(path)

    module_name = []

    #: move up untile outside package
    while True:
        path, name = os.path.split(path)
        module_name.append(name)

        if not os.path.exists(os.path.join(path, '__init__.py')):
            break

    if sys.path[0] != path:
        sys.path.insert(0, path)

    return '.'.join(module_name[::-1])


def load_module(module_name: str, raise_on_failure: bool = True) -> Optional[ModuleType]:
    try:
        __import__(module_name)
    except ImportError:
        if sys.exc_info()[-1].tb_next:
            raise RuntimeError(
                f"While importing '{module_name}', an ImportError was raised:\n\n{traceback.format_exc()}"
            )
        elif raise_on_failure:
            raise RuntimeError(f"Could not import '{module_name}'.")
        else:
            return
    return sys.modules[module_name]


def load_target(target: str, wd: Optional[Path] = None, factory: bool = False) -> Callable[..., None]:
    sys.path.insert(0, str(wd.resolve()) if wd else '')
    path, name = get_import_components(target)
    path = prepare_import(path) if path else None
    name = name or 'app'
    module = load_module(path)
    rv = module
    for element in name.split('.'):
        rv = getattr(rv, element)
    if factory:
        rv = rv()
    return rv


def load_env(files):
    for env_file in files:
        dotenv.load_dotenv(dotenv_path=env_file, override=False)


def build_env_loader():
    env = set(os.environ.keys())

    def loader(files):
        for env_file in files:
            values = dotenv.dotenv_values(dotenv_path=env_file)
            for key in set(values.keys()) - env:
                val = values[key]
                if val is not None:
                    os.environ[key] = val

    return loader
