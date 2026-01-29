from dataclasses import dataclass


@dataclass
class MainMetrics:
    spawn: int = 0
    respawn_err: int = 0
    respawn_ttl: int = 0
    respawn_rss: int = 0
