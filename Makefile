# Sync the `zain` tenant on the cubos_agent platform with ./config, using the
# `cubos-agent` CLI from the cubos-agent repo (cubos_agent_cli crate, `config` subcommand).
#
#   make pull          cloud   -> config/
#   make push          config/ -> cloud
#   make status        list out-of-sync resources
#   make diff          field-level diff of drifted resources
#   make sync          interactive per-resource pull/push/skip
#   make deploy        declarative push-all (create-or-update)
#
#   make pull TENANT=other     # override the tenant (defaults to zain)
#
#   make dab                       build dados-abertos, ship to the pc, run it there
#   make dab ARGS="--force"        # reprocess all gov open-data (atomic swap)
#   make dab ARGS="--only cnpj"    # a single schema

# Where the cubos-agent repo lives (holds the config_sync crate + server .env).
CUBOS_AGENT_REPO ?= /Users/guilherme/repos/cubos-agent
CONFIG_DIR ?= config
TENANT ?= zain

# Platform creds come from the cubos_agent server's own .env (single source of
# truth, no secret duplicated here): PORT -> URL, ROOT_API_KEY -> bearer.
# `.` (not `source`) so it works under /bin/sh.
ENV := set -a; . $(CUBOS_AGENT_REPO)/.env; set +a; \
       export CUBOS_AGENT_URL="http://localhost:$${PORT:-4000}"; \
       export CUBOS_AGENT_ROOT_KEY="$$ROOT_API_KEY";

# Build+run the binary from the cubos-agent workspace, but with this repo as the
# working dir so `--config-dir config` lands here.
RUN := cargo run -q --manifest-path $(CUBOS_AGENT_REPO)/Cargo.toml -p cubos_agent_cli -- --config-dir $(CONFIG_DIR) config
TENANT_ARG = $(if $(TENANT),--tenant $(TENANT),)

# --- dados-abertos: cross-compile -> ship -> run ON the pc ---
# The dados-abertos binary loads gov open-data into the pc's Postgres. It runs on
# the pc (local socket + fast download), not the Mac. Cross-compiled like the mcp
# deploy; the glibc-dynamic ELF runs on NixOS via nix-ld. Pass flags via ARGS.
DAB_HOST            ?= pc
DAB_DIR             ?= dab
DAB_TARGET          ?= x86_64-unknown-linux-gnu
DAB_BIN             := target/$(DAB_TARGET)/release/dados-abertos
# Env the binary reads on the pc (local socket / local Ollama), rewritten each run.
DAB_DATABASE_URL    ?= postgresql://zain@localhost/zain
DAB_OLLAMA_URL      ?= http://localhost:11434
DAB_EMBEDDING_MODEL ?= ollama/qwen3-embedding:4b-q4_K_M

.DEFAULT_GOAL := help
.PHONY: help pull push sync status diff deploy deploy-dry dab

help:
	@echo "cubos-config — tenant '$(TENANT)' <-> $(CONFIG_DIR)/ (override with TENANT=...):"
	@echo "  make pull         pull cloud -> $(CONFIG_DIR)/tenants/$(TENANT)/"
	@echo "  make push         mirror $(CONFIG_DIR)/ -> cloud (creates/updates AND deletes cloud extras)"
	@echo "  make status       list out-of-sync resources"
	@echo "  make diff         field-level diff of drifted resources"
	@echo "  make sync         interactive per-resource pull/push/skip"
	@echo "  make deploy       declarative push-all (create-or-update)"
	@echo "  make dab          build dados-abertos, ship to $(DAB_HOST), run it (ARGS=\"--force\")"

pull:
	@$(ENV) $(RUN) sync $(TENANT_ARG) --pull-all

push:
	@$(ENV) $(RUN) sync $(TENANT_ARG) --push-all --delete

sync:
	@$(ENV) $(RUN) sync $(TENANT_ARG)

status:
	@$(ENV) $(RUN) status $(TENANT_ARG)

diff:
	@$(ENV) $(RUN) diff $(TENANT_ARG)

deploy:
	@$(ENV) $(RUN) deploy $(TENANT_ARG)

deploy-dry:
	@$(ENV) $(RUN) deploy $(TENANT_ARG) --dry-run

# Build the Linux binary, copy it over (atomic rename → no ETXTBSY mid-run),
# refresh the pc-local .env, then run it there with a TTY so progress bars show.
dab:
	ulimit -n 65536 && cargo zigbuild -p dados-abertos --release --target $(DAB_TARGET)
	ssh $(DAB_HOST) 'mkdir -p $(DAB_DIR)'
	scp -q $(DAB_BIN) $(DAB_HOST):$(DAB_DIR)/dados-abertos.new
	ssh $(DAB_HOST) 'cd $(DAB_DIR) && mv -f dados-abertos.new dados-abertos && chmod +x dados-abertos && \
		printf "DATABASE_URL=%s\nOLLAMA_URL=%s\nEMBEDDING_MODEL=%s\n" \
		"$(DAB_DATABASE_URL)" "$(DAB_OLLAMA_URL)" "$(DAB_EMBEDDING_MODEL)" > .env'
	ssh -t $(DAB_HOST) 'cd $(DAB_DIR) && ./dados-abertos $(ARGS)'
