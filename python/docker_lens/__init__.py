from .docker_lens import *

try:
    from importlib.metadata import version as _get_version
    __version__ = _get_version("dlens-py")
except Exception:
    __version__ = "0.0.0"