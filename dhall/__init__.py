import contextlib
import os
from collections.abc import Generator
from typing import Any, Protocol, TypeVar

from . import dhall as _dhall
from .dhall import __version__, dump, dumps, loads

__all__ = ["__version__", "dump", "dumps", "load", "loads"]

_T_co = TypeVar("_T_co", covariant=True)


class SupportsRead(Protocol[_T_co]):
    def read(self, length: int = -1, /) -> _T_co: ...
    @property
    def name(self) -> str: ...


@contextlib.contextmanager
def remember_cwd() -> Generator[None, None, None]:
    curdir = os.getcwd()
    try:
        yield
    finally:
        os.chdir(curdir)


def load(fp: SupportsRead[str | bytes]) -> Any:
    with remember_cwd():
        newdir = os.path.dirname(fp.name)
        if newdir != "":
            os.chdir(newdir)
        return _dhall.load(fp)
