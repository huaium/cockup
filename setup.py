from __future__ import annotations

from setuptools import setup
from setuptools_scm import ScmVersion


def custom_version(version: ScmVersion) -> str:
    # Make the version the same as the tag
    return version.format_with("{tag}")


setup(use_scm_version={"version_scheme": custom_version})
