import copyreg

from ._granian import ListenerHolder as SocketHolder


copyreg.pickle(SocketHolder, lambda v: (SocketHolder, v.__getstate__()))
