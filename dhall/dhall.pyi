from __future__ import annotations

from collections.abc import Mapping, Sequence
from typing import TypeAlias

DhallValue: TypeAlias = (
    bool
    | int
    | float
    | str
    | None
    | Sequence["DhallValue"]
    | Mapping[str, "DhallValue"]
)

class DhallError(Exception): ...

__version__: str

def loads(s: str) -> DhallValue | DhallError: ...
def dumps(obj: DhallValue, sort_keys: bool = ...) -> str | DhallError: ...
def load(fp: object) -> DhallValue | DhallError: ...
def dump(obj: DhallValue, fp: object, sort_keys: bool = ...) -> None | DhallError: ...
