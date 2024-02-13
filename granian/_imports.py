try:
    import setproctitle
except ImportError:
    setproctitle = None

try:
    import watchfiles
except ImportError:
    watchfiles = None
