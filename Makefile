# Foxia-MKT — common development and release commands.
# Run `make` or `make help` to see every command.

SHELL := /bin/bash
VERSION ?=
MIN ?=
TAG ?=

.DEFAULT_GOAL := help
.PHONY: help install dev dev-web dev-full build format lint test check \
	version bump release release-status force-update test-sync

help: ## Show this help
	@echo "Usage: make <command> [VAR=value]"
	@echo ""
	@awk 'BEGIN {FS = ":.*## "} /^[a-zA-Z_-]+:.*## / {printf "  \033[36m%-16s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)
	@echo ""
	@echo "Examples:"
	@echo "  make release VERSION=patch          # 0.0.17 -> 0.0.18, commit, tag, push"
	@echo "  make release VERSION=0.1.0"
	@echo "  make force-update MIN=0.0.18        # force apps older than 0.0.18 to update"

## ---------- Setup & run ----------

install: ## Install all dependencies (pnpm, JS packages, Rust check)
	@command -v node >/dev/null || { echo "Node.js is required (see .nvmrc)"; exit 1; }
	@command -v pnpm >/dev/null || { echo "Enabling pnpm via corepack..."; corepack enable; }
	@command -v cargo >/dev/null || { echo "Rust is required: https://rustup.rs"; exit 1; }
	pnpm install

dev: ## Run the desktop app in development mode (Tauri)
	pnpm tauri dev

dev-web: ## Run only the web UI at http://localhost:12341
	pnpm dev

dev-full: ## Run the app with the sync backend and MinIO (needs Docker)
	bash scripts/dev.sh

build: ## Build the production app locally
	pnpm tauri build

## ---------- Quality ----------

format: ## Format JS and Rust code
	pnpm format

lint: ## Lint JS and Rust code
	pnpm lint

test: ## Run the Rust unit tests
	pnpm test:rust:unit

test-sync: ## Run the sync end-to-end tests (needs a MinIO binary)
	pnpm test:sync-e2e

check: ## Format, lint and Rust tests (run before every release)
	pnpm format && pnpm lint && pnpm test:rust:unit

## ---------- Version & release ----------

version: ## Show the current app version
	@node scripts/bump-version.mjs

bump: ## Bump the version locally without committing: make bump VERSION=patch|minor|major|x.y.z
	@test -n "$(VERSION)" || { echo "Usage: make bump VERSION=patch|minor|major|x.y.z"; exit 1; }
	@node scripts/bump-version.mjs "$(VERSION)"

release: ## Bump, commit, tag and push to trigger the GitHub release: make release VERSION=patch
	@set -e; \
	if [ -z "$(VERSION)" ]; then echo "Usage: make release VERSION=patch|minor|major|x.y.z"; exit 1; fi; \
	if [ "$$(git branch --show-current)" != "main" ]; then echo "Releases must be made from the main branch."; exit 1; fi; \
	if [ -n "$$(git status --porcelain)" ]; then echo "You have uncommitted changes. Commit or stash them first."; git status --short; exit 1; fi; \
	git pull --ff-only origin main; \
	CURRENT=$$(node scripts/bump-version.mjs); \
	NEXT=$$(node scripts/bump-version.mjs "$(VERSION)" --dry-run); \
	if git rev-parse "v$$NEXT" >/dev/null 2>&1; then echo "Tag v$$NEXT already exists."; exit 1; fi; \
	read -r -p "Release v$$NEXT (current $$CURRENT)? [y/N] " ANSWER; \
	if [ "$$ANSWER" != "y" ] && [ "$$ANSWER" != "Y" ]; then echo "Cancelled."; exit 1; fi; \
	node scripts/bump-version.mjs "$$NEXT" >/dev/null; \
	git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock; \
	git commit -m "chore: release v$$NEXT"; \
	git tag "v$$NEXT"; \
	git push origin main "v$$NEXT"; \
	echo ""; \
	echo "Pushed v$$NEXT. GitHub Actions is now building the release."; \
	echo "  Follow progress:  make release-status"; \
	echo "  Force old apps to update once the release is published:"; \
	echo "                    make force-update MIN=$$NEXT"

release-status: ## Show the latest GitHub release workflow runs
	@command -v gh >/dev/null || { echo "GitHub CLI is required: brew install gh"; exit 1; }
	gh run list --workflow=release.yml --limit 5

force-update: ## Force apps older than MIN to update: make force-update MIN=0.0.18 [TAG=v0.0.18]
	@test -n "$(MIN)" || { echo "Usage: make force-update MIN=x.y.z|none [TAG=vX.Y.Z]"; exit 1; }
	@bash scripts/set-min-supported-version.sh "$(MIN)" "$(TAG)"
