try:
    from importlib.metadata import version as _version

    __version__ = _version("cockup")
except ImportError:
    __version__ = "unknown"
