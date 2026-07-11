"""Apple MLX-powered Parakeet command-line interface."""

from importlib.metadata import PackageNotFoundError, version

try:
    __version__ = version("awaz-mlx")
except PackageNotFoundError:
    __version__ = "0.2.0"
