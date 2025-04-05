import copyreg

from ._granian import ListenerHolder as SocketHolder, ListenerSpec as SocketSpec


copyreg.pickle(SocketHolder, lambda v: (SocketHolder, v.__getstate__()))
copyreg.pickle(SocketSpec, lambda v: (SocketSpec, v.__getstate__()))
