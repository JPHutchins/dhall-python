from pathlib import Path

from camas import Claude, Config, Parallel, Sequential, Task


def _python_versions() -> tuple[str, ...]:
    lines = Path(".python-version").read_text(encoding="utf-8").splitlines()
    return tuple(
        line.strip() for line in lines if line.strip() and not line.startswith("#")
    )


fmt = Sequential(
    Task("cargo fmt", mutates=True),
    Task("ruff check --fix .", mutates=True),
    Task("ruff format .", mutates=True),
)

lint = Parallel(
    Task("ruff check ."),
    Task("ruff format --check ."),
)

types = Parallel(
    Task("mypy"),
    Task("pyright"),
)

rust = Parallel(
    Task("cargo fmt --check"),
    Task("cargo clippy --all-targets -- -D warnings"),
)

test = Task("pytest tests")

test_matrix = Parallel(
    Task("uv run --python {PY} pytest tests"),
    matrix={"PY": _python_versions()},
)

ci = Parallel(lint, types, rust, test)

_ = Config(default_task=ci, agent=Claude(fix=fmt, check=ci))
