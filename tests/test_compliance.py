from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING

import pytest

import dhall

if TYPE_CHECKING:
    from typing import Final

FIXTURES: Final = Path(__file__).parent / "vendor" / "dhall-lang" / "normalization"
PAIRS: Final = sorted(FIXTURES.glob("*A.dhall"))
IDS: Final = [path.name[:-7] for path in PAIRS]


def test_fixtures_present() -> None:
    assert PAIRS


@pytest.mark.parametrize("input_path", PAIRS, ids=IDS)
def test_normalization_matches_upstream(input_path: Path) -> None:
    expected_path = input_path.with_name(input_path.name[:-7] + "B.dhall")
    actual = dhall.loads(input_path.read_text(encoding="utf-8")).or_raise()
    expected = dhall.loads(expected_path.read_text(encoding="utf-8")).or_raise()
    assert actual == expected
