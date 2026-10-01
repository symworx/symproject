[project]
name = "__PROJECT_NAME__"
version = "0.1.0"
description = "__PROJECT_TITLE__"
readme = "README.md"
requires-python = ">=3.11"
dependencies = []

[project.optional-dependencies]
dev = ["pytest>=8"]

[build-system]
requires = ["setuptools>=77"]
build-backend = "setuptools.build_meta"

[tool.setuptools.packages.find]
where = ["src"]

[tool.pytest.ini_options]
testpaths = ["tests"]
pythonpath = ["src"]
