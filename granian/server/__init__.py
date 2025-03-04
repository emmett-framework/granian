from .._granian import BUILD_GIL
from .mp import MPServer as MPServer
from .mt import MTServer as MTServer


Server = MPServer if BUILD_GIL else MTServer
