try:
    import anyio
except ImportError:
    anyio = None

try:
    import dotenv
except ImportError:
    dotenv = None

try:
    import watchfiles
except ImportError:
    watchfiles = None


def import_setproctitle():
    # Importing before a fork can crash the forked child on macOS:
    # https://github.com/dvarrazzo/py-setproctitle/issues/127
    try:
        import setproctitle
    except ImportError:
        return None
    return setproctitle
