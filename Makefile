ROOT_DIR := $(dir $(abspath $(firstword $(MAKEFILE_LIST))))
DEFUSE_OUT_DIR ?= $(ROOT_DIR)res

.DEFAULT_GOAL := all
ALL_TARGETS :=

# Contract list: target=crate
CONTRACTS := \
    defuse=defuse \
    poa-factory=defuse-poa-factory \
    poa-token=defuse-poa-token \
    escrow-swap=defuse-escrow-swap \
    global-deployer=defuse-global-deployer \
    wallet=defuse-wallet \
    multi-token-receiver-stub=multi-token-receiver-stub

# Pre-process cargo metadata: flatten each package into one entry per build (base + variants)
METADATA := $(shell cargo metadata --format-version=1 | jq -c \
    '[.packages[] | select(.metadata.near.reproducible_build) | \
    .name as $$name | .manifest_path as $$mp | .metadata.near.reproducible_build as $$b | \
    {name: $$name, manifest_path: $$mp, target_postfix: "", outdir_postfix: "", \
     cmd: ($$b.container_build_command | join(" ")), \
     reproducible_cmd: "cargo near build reproducible-wasm"}, \
    ($$b.variant // {} | to_entries[] | \
     {name: $$name, manifest_path: $$mp, target_postfix: "-\(.key)", outdir_postfix: "/\(.key)", \
      cmd: (.value.container_build_command | join(" ")), \
      reproducible_cmd: "cargo near build reproducible-wasm --variant=\(.key)"})]')

# For each contract, generate targets from cached metadata
$(foreach c,$(CONTRACTS),\
    $(eval $(shell jq -rn --argjson data '$(METADATA)' \
        --arg contract '$(c)' --arg outdir '$(DEFUSE_OUT_DIR)' --arg reproducible '$(REPRODUCIBLE)' ' \
        ($$contract | split("=")) as $$e | $$e[0] as $$target | $$e[1] as $$crate | \
        $$data[] | select(.name == $$crate) | \
        "\($$target)\(.target_postfix)" as $$target_name | \
        "\($$outdir)\(.outdir_postfix)" as $$target_outdir | \
        (if $$reproducible != "" then .reproducible_cmd else .cmd end) as $$build_cmd | \
        "$$(eval .PHONY: \($$target_name))", \
        "$$(eval ALL_TARGETS += \($$target_name))", \
        "$$(eval \($$target_name):; \($$build_cmd) --manifest-path=\(.manifest_path) --out-dir=\($$target_outdir))" \
    ')))

.PHONY: all
all: $(ALL_TARGETS)

.PHONY: help
help:
	@echo "Usage: make [target] [REPRODUCIBLE=1]"
	@echo ""
	@echo "Build targets (use REPRODUCIBLE=1 for reproducible builds):"
	@$(foreach t,$(ALL_TARGETS),echo "  $(t)";)
	@echo ""
	@echo "Other targets:"
	@echo "  all              Build all contracts (default)"
	@echo "  clean            Remove build artifacts and cargo clean"
	@echo "  clean-out-dir    Remove output directory only"
	@echo "  test             Run all workspace tests"
	@echo "  clippy           Run clippy lints"
	@echo "  metadata         Print cargo metadata JSON"
	@echo "  help             Show this help"

.PHONY: metadata
metadata:
	@echo '$(METADATA)' | jq .

.PHONY: clean-out-dir
clean-out-dir:
	rm -rf $(DEFUSE_OUT_DIR)

.PHONY: clean
clean: clean-out-dir
	cargo clean

.PHONY: test
test:
	cargo test --workspace --all-targets

.PHONY: clippy
clippy:
	cargo clippy --workspace --all-targets --no-deps
