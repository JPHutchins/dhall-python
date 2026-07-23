from __future__ import annotations

from typing import TYPE_CHECKING

import pytest

import dhall

if TYPE_CHECKING:
    from typing import Final

    from dhall import DhallValue

LOADS: Final[list[tuple[str, DhallValue]]] = [
    ("True", True),
    ("False", False),
    ("0", 0),
    ("42", 42),
    ("+7", 7),
    ("-5", -5),
    ("3.5", 3.5),
    ("-2.0", -2.0),
    ('"hello"', "hello"),
    ('""', ""),
    ("Some 1", 1),
    ("None Natural", None),
    ("[1, 2, 3]", [1, 2, 3]),
    ("[] : List Natural", []),
    ('{ a = 1, b = "x" }', {"a": 1, "b": "x"}),
    ("{=}", {}),
    ("< A | B >.A", "A"),
    ("< A : Natural | B >.A 1", 1),
    ("{ outer = { inner = [1, 2] } }", {"outer": {"inner": [1, 2]}}),
]

DUMPS: Final[list[tuple[DhallValue, str]]] = [
    (True, "True"),
    (False, "False"),
    (0, "0"),
    (42, "42"),
    (-5, "-5"),
    (3.5, "3.5"),
    ("hello", '"hello"'),
    ("", '""'),
    (None, "{=}"),
    ([1, 2, 3], "[1, 2, 3]"),
    ({"a": 1}, "{ a = 1 }"),
    (
        {"keyA": 81, "keyB": True, "keyC": "value"},
        '{ keyA = 81, keyB = True, keyC = "value" }',
    ),
]

ROUND_TRIP: Final[list[DhallValue]] = [
    True,
    False,
    0,
    42,
    -5,
    3.5,
    "hello",
    "",
    [1, 2, 3],
    {"a": 1, "b": "x"},
    {"outer": {"inner": [1, 2, 3]}},
]


@pytest.mark.parametrize(("source", "expected"), LOADS)
def test_loads(source: str, expected: DhallValue) -> None:
    actual = dhall.loads(source).or_raise()
    assert actual == expected
    assert type(actual) is type(expected)


@pytest.mark.parametrize(("value", "expected"), DUMPS)
def test_dumps(value: DhallValue, expected: str) -> None:
    assert dhall.dumps(value).or_raise() == expected


@pytest.mark.parametrize("value", ROUND_TRIP)
def test_round_trip(value: DhallValue) -> None:
    assert dhall.loads(dhall.dumps(value).or_raise()).or_raise() == value


@pytest.mark.parametrize(
    "source", ["{ a = }", "1 + True", "\\(x : Natural) -> x", "[1, -1]", "oops"]
)
def test_loads_rejects(source: str) -> None:
    assert isinstance(dhall.loads(source), dhall.Err)


@pytest.mark.parametrize("value", [object(), {1, 2, 3}, 1j, b"bytes"])
def test_dumps_rejects_unserializable(value: object) -> None:
    assert isinstance(dhall.dumps(value), dhall.Err)  # type: ignore[arg-type]  # pyright: ignore[reportArgumentType]


def test_sort_keys_is_currently_a_noop() -> None:
    record: DhallValue = {"b": 1, "a": 2}
    assert (
        dhall.dumps(record, sort_keys=True).or_raise() == dhall.dumps(record).or_raise()
    )
