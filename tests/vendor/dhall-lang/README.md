# Vendored `dhall-lang` acceptance fixtures

`normalization/` holds a curated subset of the [`dhall-lang`][dhall-lang]
`tests/normalization/success` suite, pinned at commit
`4548bb08b5fc94b761d5b3c17e9790b960fabf24`.

Each `<name>A.dhall` is an expression and `<name>B.dhall` its normal form.
`tests/test_compliance.py` asserts `loads(A) == loads(B)` — that this binding
resolves each upstream expression to the same Python value as its already
pre-normalized form.

Included are the import-free pairs whose normal form is a `SimpleValue` — the
surface this binding exposes. Pairs resolving to functions, types, or other
non-`SimpleValue` results, and pairs with imports, are excluded. Upstream paths
are flattened into the filename (`unit/Foo` → `unit__Foo`).

Licensed under BSD-3-Clause (see `LICENSE`), © Gabriella Gonzalez and the
`dhall-lang` contributors.

[dhall-lang]: https://github.com/dhall-lang/dhall-lang
