#!/usr/bin/env python
"""Repository-local wrapper for ``garganta-evolve``.

Run from the repository root after installing the audio extra:

    .venv/bin/python mus-rs/tools/evolve_garganta_throat.py voice.wav \
      --out paper/garganta-speaker/voice
"""

from mus_analysis.garganta_cli import main


if __name__ == "__main__":
    raise SystemExit(main())
