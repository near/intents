ROOT_DIR := $(dir $(abspath $(firstword $(MAKEFILE_LIST))))
DEFUSE_OUT_DIR ?= $(ROOT_DIR)res
MAKE_OUT_DIR_PREFIX ?= $(ROOT_DIR)target/makenear
MAKE_OUT_DIR = $(eval MAKE_OUT_DIR := $(shell mkdir -p $(MAKE_OUT_DIR_PREFIX) $(DEFUSE_OUT_DIR) && mktemp -d -p $(MAKE_OUT_DIR_PREFIX)))$(MAKE_OUT_DIR)


.DEFAULT_GOAL := all
CONTRACT_CRATES := \
    defuse \
    defuse-escrow-swap \
    defuse-global-deployer \
    defuse-poa-factory \
    defuse-poa-token \
    defuse-wallet \
    multi-token-receiver-stub

ALL_TARGETS :=

# Generate all build targets from cargo metadata, filtered to CONTRACT_CRATES
$(eval $(shell cargo metadata --format-version=1 | jq -rn \
    --arg outdir '$(DEFUSE_OUT_DIR)' --arg reproducible '$(REPRODUCIBLE)' --arg makedir '$$$$(MAKE_OUT_DIR)' \
    --arg crates '$(CONTRACT_CRATES)' --arg reproducible_cmd 'cargo near build reproducible-wasm' ' \
    ($$crates | split(" ") | map(select(length > 0))) as $$allowed | \
    [inputs][0].packages[] | select(.metadata.near.reproducible_build) | select(.name as $$n | $$allowed | any(. == $$n)) | \
    .name as $$name | .manifest_path as $$mp | .metadata.near.reproducible_build as $$b | \
    ($$name | gsub("-"; "_")) as $$wasm_base | \
    ([$$b.variant // {} | to_entries[] | "\($$name)/\(.key)"] | join(" ")) as $$variant_targets | \
    ({"": $$b} + ($$b.variant // {}) | to_entries[] | \
     .key as $$vkey | .value as $$vval | \
     ("\($$name)/\($$vkey)" | rtrimstr("/")) as $$tname | \
     "\($$makedir)/\($$tname)" as $$tout | \
     (if $$vkey != "" then " --variant=\($$vkey)" else "" end) as $$variant | \
     ($$reproducible_cmd + $$variant) as $$reproducible_cmd | \
     ($$vval.container_build_command | join(" ")) as $$non_reproducible_cmd | \
     (if $$reproducible != "" then $$reproducible_cmd else $$non_reproducible_cmd end) as $$cmd | \
     (if $$vkey == "" then "" else ".\($$vkey)" end) as $$suffix | \
     "$$(eval .PHONY: \($$tname))", \
     "$$(eval ALL_TARGETS += \($$tname))", \
     "$$(eval \($$tname)::;  \($$cmd) --manifest-path=\($$mp) --out-dir=\($$tout))", \
     "$$(eval \($$tname)::; -@cp -v \($$tout)/\($$wasm_base).wasm \($$outdir)/\($$wasm_base)\($$suffix).wasm)", \
     "$$(eval \($$tname)::; -@cp -v \($$tout)/\($$wasm_base)_abi.json \($$outdir)/\($$wasm_base)\($$suffix).abi.json)" \
    ), \
    "$$(eval .PHONY: \($$name)/all)", \
    "$$(eval ALL_TARGETS +=  \($$name)/all)", \
    "$$(eval \($$name)/all: \($$name) \($$variant_targets))" \
    '))

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
	@echo "  help             Show this help"

.PHONY: clean-out-dir
clean-out-dir:
	rm -rf $(DEFUSE_OUT_DIR) $(MAKE_OUT_DIR_PREFIX)

.PHONY: clean
clean: clean-out-dir
	cargo clean

.PHONY: test
test:
	cargo test --workspace --all-targets

.PHONY: clippy
clippy:
	cargo clippy --workspace --all-targets --no-deps
