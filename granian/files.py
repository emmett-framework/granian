from dataclasses import dataclass


@dataclass
class StaticFilesSettings:
    """Configuration for static file serving.

    Attributes:
        mounts: List of (route_prefix, filesystem_path) tuples for serving static files.
        dir_to_file: Optional filename to serve when a directory is requested (e.g., 'index.html').
        expires: Cache-Control max-age value in seconds (as string), or None to disable.
        precompressed: Whether to serve pre-compressed sidecar files (.br, .gz, .zst).
    """

    mounts: list[tuple[str, str]]
    dir_to_file: str | None = None
    expires: str | None = None
    precompressed: bool = False
