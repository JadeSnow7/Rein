#!/bin/bash
set -euo pipefail
/private/tmp/rein-python-part1-venv/bin/python -m unittest discover -s python/part1/tests -v
python3 records/REIN-PYTHON-PART1-20260922/checks/verify_cli.py
/private/tmp/rein-python-part1-venv/bin/python records/REIN-PYTHON-PART1-20260922/checks/verify_sdk_flow.py
/private/tmp/rein-python-part1-venv/bin/python records/REIN-PYTHON-PART1-20260922/checks/verify_shortest_sdk.py
python3 records/REIN-PYTHON-PART1-20260922/checks/freeze_s1.py --check
npm run book:check
node --import tsx --test docs/.vitepress/theme/sourceVersionState.test.ts
npm run build
python3 records/REIN-PYTHON-PART1-20260922/checks/verify_site.py
node scripts/book-links.mjs
git diff --check -- README.md docs/index.md docs/about.md docs/toc.md docs/.vitepress/config.mts
