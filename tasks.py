import sys
import sysconfig
from pathlib import Path

from camas import Claude, Config, Parallel, Sequential, Task

fmt_rust = Sequential(
    Task(
        "cargo clippy --fix --allow-dirty --allow-staged --all-targets",
        mutates=True,
    ),
    Task("cargo fmt", mutates=True),
)
fmt_python = Sequential(
    Task("uv run ruff check --fix .", mutates=True),
    Task("uv run ruff format .", mutates=True),
)
fmt = Parallel(fmt_rust, fmt_python)

lint = Parallel(Task("uv run ruff check ."), Task("uv run ruff format --check ."))
types = Parallel(Task("uv run mypy"), Task("uv run pyright"))
pytest = Task("uv run pytest -v tests")
check = Parallel(lint, types, pytest)

matrix = Sequential(
    check,
    matrix={
        "PY": tuple(
            line.strip()
            for line in Path(".python-version").read_text(encoding="utf-8").splitlines()
            if line.strip() and not line.startswith("#")
        )
    },
    env={"UV_PROJECT_ENVIRONMENT": ".camas/.venv-{PY}", "UV_PYTHON": "{PY}"},
)

rust = Parallel(
    Task("cargo fmt --check"), Task("cargo clippy --all-targets -- -D warnings")
)
actions = Task("uv run actionlint")

_libdir = sysconfig.get_config_var("LIBDIR") or ""
nextest = Task(
    "uv run cargo nextest run --no-default-features --features auto-initialize",
    env={
        "PYTHONHOME": sys.base_prefix,
        "LD_LIBRARY_PATH": _libdir,
        "DYLD_LIBRARY_PATH": _libdir,
    },
)

develop = Task("uv run maturin develop")
build = Task("uv run maturin build --release --compatibility linux")

verify = Parallel(check, rust, actions, nextest)
all = Sequential(fmt, verify)
ci = Parallel(matrix, rust, actions, nextest, build)

_ = Config(default_task=all, github_task=ci, agent=Claude(fix=fmt, check=verify))
