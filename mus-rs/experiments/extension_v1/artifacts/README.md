# Recorded Extension-v1 receipts

This directory keeps the compact, human-reviewable JSON evidence used by the
research note:

- the direct active-cable negative result;
- exact Clifford algebra and determinant checks;
- closed-loop development;
- learned closed-token runs and commuting controls;
- one representative five-string geometric realization;
- compact summaries of the geometry and morphology sweeps.

The companion downloadable delivery bundle contains the full raw sweep rows,
the learned `model.pt` checkpoint, and generated plots. Those generated binary
artifacts are intentionally omitted from the Git branch; every experiment that
produced them is exposed through the package CLI and can be replayed.

Compact sweep receipts carry explicit fields reporting how many raw rows were
omitted. No headline number in `RESULTS.md` depends on an unreported aggregate.
