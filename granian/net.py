import copyreg

from . import _tcp

SocketHolder = _tcp.ListenerHolder
copyreg.pickle(
    SocketHolder,
    lambda v: (SocketHolder, v.__getstate__())
)
