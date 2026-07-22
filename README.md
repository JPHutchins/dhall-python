[![Dhall logo](https://github.com/dhall-lang/dhall-lang/blob/master/img/dhall-logo.svg)](https://dhall-lang.org/)

[![Maintenance](https://img.shields.io/badge/Maintained%3F-yes-green.svg)](https://GitHub.com/s-zeng/dhall-python/graphs/commit-activity)
[![CI status](https://github.com/s-zeng/dhall-python/workflows/CI/badge.svg)](https://github.com/s-zeng/dhall-python/actions)
[![PyPI version shields.io](https://img.shields.io/pypi/v/dhall.svg)](https://pypi.python.org/pypi/dhall/)
[![PyPI downloads](https://img.shields.io/pypi/dm/dhall.svg)](https://pypistats.org/packages/dhall)

Dhall is a programmable configuration language optimized for
maintainability.

You can think of Dhall as: JSON + functions + types + imports

Note that while Dhall is programmable, Dhall is not Turing-complete.  Many
of Dhall's features take advantage of this restriction to provide stronger
safety guarantees and more powerful tooling.

You can try the language live in your browser by visiting the official website:

* [https://dhall-lang.org](http://dhall-lang.org/)

# `dhall-python`

`dhall-python` contains [Dhall][dhall-lang] bindings for Python using the 
[rust][dhall-rust] implementation. It is meant to be used to integrate Dhall 
into your python applications.

If you only want to convert Dhall to/from JSON or YAML, you should use the
official tooling instead; instructions can be found
[here](https://docs.dhall-lang.org/tutorials/Getting-started_Generate-JSON-or-YAML.html).

## Usage

Install using pip:

```shell
pip install dhall
```

Supports the following:

- Operating Systems
  - Windows
  - Mac OS
  - Linux (manylinux_2_28_x86_64)
- Python versions (one `abi3` wheel per OS covers the whole range)
  - 3.10
  - 3.11
  - 3.12
  - 3.13
  - 3.14
  - 3.15

Python 3.7 through 3.9 support is available in older versions of dhall-python.

dhall-python offers a `json`-style API. `loads`, `dumps`, `load`, and `dump`
return a typed `Result` (`Ok` or `Err`) rather than raising; call `.or_raise()`
to get the value (raising on failure), `.unwrap_or(default)`, or pattern-match:

```python
>>> import dhall
>>> dhall.dumps({"keyA": 81, "keyB": True, "keyC": "value"}).or_raise()
'{ keyA = 81, keyB = True, keyC = "value" }'
>>> dhall.loads("""{ keyA = 81, keyB = True, keyC = "value" }""").or_raise()
{'keyA': 81, 'keyB': True, 'keyC': 'value'}
```

# License

dhall-python is licensed under either of

- Apache License, Version 2.0, (LICENSE-APACHE or
  http://www.apache.org/licenses/LICENSE-2.0)
- MIT license (LICENSE-MIT or http://opensource.org/licenses/MIT)

at your option.

# Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in python-dhall by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.

All contributions are welcome! If you spot any bugs, or have any requests, 
issues and PRs are always welcome.

# Developer guide

This project uses [uv](https://docs.astral.sh/uv/) for managing the development environment. If you don't have it installed, follow the [installation guide](https://docs.astral.sh/uv/getting-started/installation/).

The project requires the latest `stable` version of Rust.

Install it via `rustup`:

```
rustup install stable
```

If you have already installed the `stable` version, make sure it is up-to-date:

```
rustup update stable
```

After that, build the extension and run the full check suite (ruff, mypy, pyright, clippy, and the tests) with:

```
uv sync
uv run camas
```

Tasks are defined in `tasks.py`; `uv run camas --list` shows them and `uv run camas fmt` auto-formats.


[dhall-rust]: https://github.com/Nadrieril/dhall-rust
[dhall-lang]: https://dhall-lang.org
