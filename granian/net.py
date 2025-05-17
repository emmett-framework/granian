import copyreg

from ._granian import ListenerSpec as SocketSpec, SocketHolder


copyreg.pickle(SocketHolder, lambda v: (SocketHolder, v.__getstate__()))
copyreg.pickle(SocketSpec, lambda v: (SocketSpec, v.__getstate__()))
