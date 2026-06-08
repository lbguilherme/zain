# Sync the `zain` tenant on the cubos_agent platform with ./config, using the
# `cubos-config` binary from the cubos-agent repo (config_sync crate).
#
#   make pull          cloud   -> config/
#   make push          config/ -> cloud
#   make status        list out-of-sync resources
#   make diff          field-level diff of drifted resources
#   make sync          interactive per-resource pull/push/skip
#   make deploy        declarative push-all (create-or-update)
#   make build         (re)build the cubos-config binary
#
#   make pull TENANT=other     # override the tenant (defaults to zain)

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
RUN := cargo run -q --manifest-path $(CUBOS_AGENT_REPO)/Cargo.toml -p config_sync -- --config-dir $(CONFIG_DIR)
TENANT_ARG = $(if $(TENANT),--tenant $(TENANT),)

.DEFAULT_GOAL := help
.PHONY: help pull push sync status diff deploy deploy-dry build

help:
	@echo "cubos-config — tenant '$(TENANT)' <-> $(CONFIG_DIR)/ (override with TENANT=...):"
	@echo "  make pull         pull cloud -> $(CONFIG_DIR)/tenants/$(TENANT)/"
	@echo "  make push         mirror $(CONFIG_DIR)/ -> cloud (creates/updates AND deletes cloud extras)"
	@echo "  make status       list out-of-sync resources"
	@echo "  make diff         field-level diff of drifted resources"
	@echo "  make sync         interactive per-resource pull/push/skip"
	@echo "  make deploy       declarative push-all (create-or-update)"
	@echo "  make build        (re)build the cubos-config binary"

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

build:
	cargo build --manifest-path $(CUBOS_AGENT_REPO)/Cargo.toml -p config_sync
