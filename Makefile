ROOT_DIR := $(dir $(abspath $(firstword $(MAKEFILE_LIST))))
DEFUSE_OUT_DIR ?= $(ROOT_DIR)res
MAKE_OUT_DIR ?= $(ROOT_DIR)target/makenear

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
    --arg outdir '$(DEFUSE_OUT_DIR)' --arg reproducible '$(REPRODUCIBLE)' \
    --arg crates '$(CONTRACT_CRATES)' ' \
    ($$crates | split(" ") | map(select(length > 0))) as $$allowed | \
    [inputs][0].packages[] | select(.metadata.near.reproducible_build) | select(.name as $$n | $$allowed | any(. == $$n)) | \
    .name as $$name | .manifest_path as $$mp | .metadata.near.reproducible_build as $$b | \
    (if $$reproducible != "" then "cargo near build reproducible-wasm" else ($$b.container_build_command | join(" ")) end) as $$base_cmd | \
    ($$name | gsub("-"; "_")) as $$wasm_base | \
    ([$$b.variant // {} | to_entries[] | "\($$name)/\(.key)"] | join(" ")) as $$variant_targets | \
    "$(MAKE_OUT_DIR)/\($$name)" as $$default_outdir | \
    "$$(eval .PHONY: \($$name))", \
    "$$(eval ALL_TARGETS += \($$name))", \
    ("$$(eval \($$name):;" + \
        " rm -rf \($$default_outdir)" + \
        " && mkdir -p \($$outdir)" + \
        " && \($$base_cmd) --manifest-path=\($$mp) --out-dir=\($$default_outdir)" + \
        " && cp \($$default_outdir)/\($$wasm_base).wasm \($$outdir)/\($$wasm_base).wasm" + \
        " && cp \($$default_outdir)/\($$wasm_base)_abi.json \($$outdir)/\($$wasm_base)_abi.json)"), \
    ($$b.variant // {} | to_entries[] | \
     "\($$name)/\(.key)" as $$vname | \
     "$(MAKE_OUT_DIR)/\($$name)____\(.key)" as $$variant_outdir | \
     (if $$reproducible != "" then "cargo near build reproducible-wasm --variant=\(.key)" else (.value.container_build_command | join(" ")) end) as $$vcmd | \
     "$$(eval .PHONY: \($$vname))", \
     "$$(eval ALL_TARGETS += \($$vname))", \
     ("$$(eval \($$vname):;" + \
         " rm -rf \($$variant_outdir)" + \
         " && mkdir -p \($$outdir)" + \
         " && \($$vcmd) --manifest-path=\($$mp) --out-dir=\($$variant_outdir)" + \
         " && cp \($$variant_outdir)/\($$wasm_base).wasm \($$outdir)/\($$wasm_base).\(.key).wasm" + \
         " && cp \($$variant_outdir)/\($$wasm_base)_abi.json \($$outdir)/\($$wasm_base).\(.key)_abi.json)") \
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
