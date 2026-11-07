# Secrets
.env

# Restricted extracts (never commit)
data/private/
data/local/
data/open/

# Local catalogs and dumps (schema SQL stays tracked)
*.sqlite
*.sqlite-journal
*.sqlite-wal
*.sqlite-shm
*.dump
*.pgdump
*.sql.gz

# Generated analysis artifacts (keep READMEs and .gitkeep).
# Directories must stay un-ignored or Git will not re-include files inside them.
results/**
!results/**/
!results/README.md
!results/**/README.md
!results/**/.gitkeep

# Python
__pycache__/
*.py[cod]
*.egg-info/
.venv/

# Agent trees (do not commit)
.agents/
.grok/
.claude/
.codex/
.symkit/
