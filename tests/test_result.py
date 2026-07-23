from __future__ import annotations

from collections.abc import Mapping, Sequence

import pytest
from typing_extensions import assert_never, assert_type

import dhall
from dhall import DhallError, DhallValue, Err, Ok, Result


def describe(value: DhallValue) -> str:
    match value:
        case bool():
            return "bool"
        case int():
            return "int"
        case float():
            return "float"
        case str():
            return "str"
        case None:
            return "none"
        case Mapping():
            return "mapping"
        case Sequence():
            return "sequence"
        case _:
            assert_never(value)


def outcome(result: Result[DhallValue]) -> str:
    match result:
        case Ok(value):
            return f"ok:{describe(value)}"
        case Err(error):
            return f"err:{type(error).__name__}"
        case _:
            assert_never(result)


@pytest.mark.parametrize(
    ("source", "expected"),
    [
        ("True", "bool"),
        ("42", "int"),
        ("-7", "int"),
        ("3.5", "float"),
        ('"hi"', "str"),
        ("None Natural", "none"),
        ("{ a = 1 }", "mapping"),
        ("[1, 2, 3]", "sequence"),
    ],
)
def test_describe_covers_every_variant(source: str, expected: str) -> None:
    assert describe(dhall.loads(source).or_raise()) == expected


def test_outcome_ok() -> None:
    assert outcome(dhall.loads("{ a = 1 }")) == "ok:mapping"


def test_outcome_err() -> None:
    assert outcome(dhall.loads("{ a = }")) == "err:DhallError"


def test_ok_is_ok_and_unwraps() -> None:
    result = dhall.loads("[1, 2, 3]")
    assert isinstance(result, Ok)
    assert result.is_ok()
    assert result.or_raise() == [1, 2, 3]


def test_ok_map_transforms_value() -> None:
    doubled = dhall.loads("2").map(lambda value: [value, value])
    assert isinstance(doubled, Ok)
    assert doubled.or_raise() == [2, 2]


def test_err_holds_dhall_error() -> None:
    result = dhall.loads("1 + True")
    assert isinstance(result, Err)
    assert not result.is_ok()
    assert isinstance(result.error, DhallError)


def test_err_or_raise_raises() -> None:
    result = dhall.loads("{ a = }")
    with pytest.raises(DhallError):
        result.or_raise()


def test_err_unwrap_or_returns_default() -> None:
    result = dhall.loads("{ a = }")
    assert result.unwrap_or("fallback") == "fallback"


def test_err_map_passes_error_through() -> None:
    mapped = dhall.loads("{ a = }").map(lambda value: [value])
    assert isinstance(mapped, Err)


def test_public_functions_are_typed() -> None:
    assert_type(dhall.loads("1"), Result[DhallValue])
    assert_type(dhall.dumps(1), Result[str])


def test_ok_and_err_are_typed() -> None:
    ok = Ok(1)
    assert_type(ok, Ok[int])
    assert_type(ok.or_raise(), int)
    assert_type(ok.unwrap_or(0), int)
    assert_type(ok.map(lambda n: [n]), Ok[list[int]])

    err: Err[int] = Err(DhallError("boom"))
    assert_type(err, Err[int])
    assert_type(err.unwrap_or(0), int)
    assert_type(err.map(lambda n: [n]), Err[list[int]])
