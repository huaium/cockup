# we presume installed build dependencies
from __future__ import annotations

from setuptools import setup
from setuptools_scm import ScmVersion


def custom_version(version: ScmVersion) -> str:
    from setuptools_scm.version import guess_next_version

    return version.format_next_version(guess_next_version, "{guessed}")


setup(use_scm_version={"version_scheme": custom_version})
