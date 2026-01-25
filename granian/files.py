from dataclasses import dataclass


@dataclass
class StaticFilesSettings:
    """Configuration for static file serving.

    Attributes:
        mount: The filesystem path to serve static files from.
        prefix: The URL path prefix for static file routes.
        expires: Cache-Control max-age value in seconds (as string), or None to disable.
        precompressed: Whether to serve pre-compressed sidecar files (.br, .gz, .zst).
    """

    mount: str
    prefix: str = '/static'
    expires: str | None = None
    precompressed: bool = False
