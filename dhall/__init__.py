from __future__ import annotations

import contextlib
import os
from collections.abc import Callable, Generator, Mapping, Sequence
from typing import Generic, NoReturn, Protocol, TypeAlias, TypeVar

from . import dhall as _dhall
from .dhall import DhallError
from .dhall import __version__ as __version__

DhallValue: TypeAlias = (
    bool
    | int
    | float
    | str
    | None
    | Sequence["DhallValue"]
    | Mapping[str, "DhallValue"]
)

_T = TypeVar("_T")
_U = TypeVar("_U")
_T_co = TypeVar("_T_co", covariant=True)
_T_contra = TypeVar("_T_contra", contravariant=True)


class SupportsRead(Protocol[_T_co]):
    def read(self, length: int = -1, /) -> _T_co: ...
    @property
    def name(self) -> str: ...


class SupportsWrite(Protocol[_T_contra]):
    def write(self, data: _T_contra, /) -> object: ...


class Ok(Generic[_T]):
    __match_args__ = ("value",)

    def __init__(self, value: _T) -> None:
        self.value = value

    def is_ok(self) -> bool:
        return True

    def or_raise(self) -> _T:
        return self.value

    def unwrap_or(self, default: _T) -> _T:
        return self.value

    def map(self, func: Callable[[_T], _U]) -> Ok[_U]:
        return Ok(func(self.value))


class Err(Generic[_T]):
    __match_args__ = ("error",)

    def __init__(self, error: DhallError) -> None:
        self.error = error

    def is_ok(self) -> bool:
        return False

    def or_raise(self) -> NoReturn:
        raise self.error

    def unwrap_or(self, default: _T) -> _T:
        return default

    def map(self, func: Callable[[_T], _U]) -> Err[_U]:
        return Err(self.error)


Result: TypeAlias = Ok[_T] | Err[_T]


def _wrap(raw: _T | DhallError) -> Result[_T]:
    return Err(raw) if isinstance(raw, DhallError) else Ok(raw)


@contextlib.contextmanager
def remember_cwd() -> Generator[None, None, None]:
    curdir = os.getcwd()
    try:
        yield
    finally:
        os.chdir(curdir)


def loads(s: str) -> Result[DhallValue]:
    return _wrap(_dhall.loads(s))


def dumps(obj: DhallValue, sort_keys: bool = False) -> Result[str]:
    return _wrap(_dhall.dumps(obj, sort_keys))


def dump(
    obj: DhallValue, fp: SupportsWrite[str], sort_keys: bool = False
) -> Result[None]:
    return _wrap(_dhall.dump(obj, fp, sort_keys))


def load(fp: SupportsRead[str | bytes]) -> Result[DhallValue]:
    with remember_cwd():
        newdir = os.path.dirname(fp.name)
        if newdir:
            os.chdir(newdir)
        return _wrap(_dhall.load(fp))
