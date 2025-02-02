from .._granian import BUILD_GIL


if BUILD_GIL:
    from .mp import MPServer as Server
else:
    from .mt import MTServer as Server  # noqa: F401
