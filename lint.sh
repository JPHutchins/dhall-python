#!/usr/bin/env bash

cargo fmt
ruff format .
ruff check --fix .
