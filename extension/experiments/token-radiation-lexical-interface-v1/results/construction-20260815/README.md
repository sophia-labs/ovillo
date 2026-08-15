# Committed construction result

This directory contains only configuration, summary, and digest receipts.  It
does not contain model weights or source corpora.

```text
scientific digest
4072fe49daa7dd353c85b0aac74eb31e6f2812739717ea94daf40e99339662a2
```

Reproduce with:

```bash
token-radiation-lexical-interface-v1 run \
  configs/construction.json \
  runs/reproduction
```

Verify the committed result with:

```bash
token-radiation-lexical-interface-v1 verify \
  results/construction-20260815
```
