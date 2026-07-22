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
rust = Parallel(
    Task("cargo fmt --check"), Task("cargo clippy --all-targets -- -D warnings")
)
actions = Task("uv run actionlint")
test = Task("uv run pytest -v tests")

develop = Task("uv run maturin develop")
build = Task("uv run maturin build --release")

ci = Parallel(lint, types, rust, actions, test)
all = Sequential(fmt, ci)

test_matrix = Parallel(
    Task("uv run --python {PY} pytest -v tests"),
    matrix={
        "PY": tuple(
            line.strip()
            for line in Path(".python-version").read_text(encoding="utf-8").splitlines()
            if line.strip() and not line.startswith("#")
        )
    },
)

_ = Config(default_task=all, github_task=ci, agent=Claude(fix=fmt, check=ci))
