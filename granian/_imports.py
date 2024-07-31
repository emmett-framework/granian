try:
    import setproctitle
except ImportError:
    setproctitle = None

try:
    import watchfiles
    from watchfiles import BaseFilter
except ImportError:
    watchfiles = None
    BaseFilter = None
