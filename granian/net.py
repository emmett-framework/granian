import copyreg
import sys

from ._granian import ListenerSpec as SocketSpec, SocketHolder


copyreg.pickle(SocketHolder, lambda v: (SocketHolder, v.__getstate__()))
copyreg.pickle(SocketSpec, lambda v: (SocketSpec, v.__getstate__()))

if sys.platform != 'win32':
    from ._granian import UnixListenerSpec as UnixSocketSpec, FdListenerSpec as FdSocketSpec

    copyreg.pickle(UnixSocketSpec, lambda v: (UnixSocketSpec, v.__getstate__()))
    copyreg.pickle(FdSocketSpec, lambda v: (FdSocketSpec, v.__getstate__()))
else:
    UnixSocketSpec = None
    FdSocketSpec = None
