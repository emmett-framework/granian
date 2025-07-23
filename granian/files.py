from dataclasses import dataclass
from typing import Optional


@dataclass
class StaticFilesSettings:
    mount: str
    prefix: str = '/static'
    expires: Optional[str] = None
