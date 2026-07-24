import dhall


def test_docs() -> None:
    assert (
        dhall.dumps({"keyA": 81, "keyB": True, "keyC": "value"}).or_raise()
        == '{ keyA = 81, keyB = True, keyC = "value" }'
    )
    assert dhall.loads('{ keyA = 81, keyB = True, keyC = "value" }').or_raise() == {
        "keyA": 81,
        "keyB": True,
        "keyC": "value",
    }
