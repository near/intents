ROOT_DIR := $(dir $(firstword $(MAKEFILE_LIST)))
DEFUSE_OUT_DIR ?= $(ROOT_DIR)res
MAKE_OUT_DIR_PREFIX ?= $(ROOT_DIR)target/makenear
MAKE_OUT_DIR = $(eval MAKE_OUT_DIR := $(shell mkdir -p $(MAKE_OUT_DIR_PREFIX) $(DEFUSE_OUT_DIR) && mktemp -d -p $(MAKE_OUT_DIR_PREFIX)))$(MAKE_OUT_DIR)

# Crates where every enum variant is feature-gated, requiring at least one
# variant feature. These need --feature-powerset + --at-least-one-of instead
# of --each-feature (which tests features in isolation).
# Format: crate=feature1,feature2,...
CRATES_AT_LEAST_ONE_VARIANT := \
    defuse-token-id=nep141,nep171,nep245,imt \
    defuse-ton-connect=text,binary,cell \
    defuse-escrow-swap=nep141,nep245

# Testing crates that cannot compile for wasm32-unknown-unknown.
# defuse-randomness uses rand/getrandom which lacks wasm32 support;
# it only reaches the defuse contract via dev-dependencies, never in the WASM binary.
CRATES_HOST_ONLY := \
    defuse-test-utils \
    defuse-sandbox \
    defuse-randomness \
    defuse-tests

crate_name = $(firstword $(subst =, ,$1))
crate_features = $(lastword $(subst =, ,$1))


RUSTFLAGS_CHECK = -D warnings
# --cfg clippy: near-sdk compile_error!s on host unless one of its allowed cfgs is set
CARGO_CHECK_HOST = RUSTFLAGS='$(RUSTFLAGS_CHECK) --cfg clippy' cargo hack check --exclude-features contract --no-dev-deps
CARGO_CHECK_WASM = RUSTFLAGS='$(RUSTFLAGS_CHECK)' cargo hack check --target wasm32-unknown-unknown --exclude-features abi --exclude-features near-api-types --exclude-features near-api --no-dev-deps

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
    . as {$$name, manifest_path: $$mp} | .metadata.near.reproducible_build as $$b | \
    ($$name | gsub("-"; "_")) as $$wasm_base | \
    "$$(eval .PHONY: \($$name)/all)", \
    "$$(eval ALL_TARGETS +=  \($$name)/all)", \
    "$$(eval \($$name)/all:: \($$name))", \
    ({"": $$b} + ($$b.variant // {}) | to_entries[] | \
     . as {key: $$vkey, value: $$vval} | \
     ("\($$name)/\($$vkey)" | rtrimstr("/")) as $$tname | \
     "\($$makedir)/\($$tname)" as $$tout | \
     (if $$vkey != "" then " --variant=\($$vkey)" else "" end) as $$variant | \
     ($$reproducible_cmd + $$variant) as $$reproducible_cmd | \
     ($$vval.container_build_command | join(" ")) as $$non_reproducible_cmd | \
     (if $$reproducible != "" then $$reproducible_cmd else $$non_reproducible_cmd end) as $$cmd | \
     (if $$vkey == "" then "" else ".\($$vkey)" end) as $$suffix | \
     "$$(eval .PHONY: \($$tname))", \
     "$$(eval ALL_TARGETS += \($$tname))", \
     "$$(eval \($$name)/all:: \($$tname))", \
     "$$(eval \($$tname)::;  \($$cmd) --manifest-path=\($$mp) --out-dir=\($$tout))", \
     "$$(eval \($$tname)::; -@cp -v \($$tout)/\($$wasm_base).wasm \($$outdir)/\($$name)\($$suffix).wasm)", \
     "$$(eval \($$tname)::; -@cp -v \($$tout)/\($$wasm_base)_abi.json \($$outdir)/\($$name)\($$suffix).abi.json)" \
    )'))

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
	@echo "  all                 Build all contracts (default)"
	@echo "  clean               Remove build artifacts and cargo clean"
	@echo "  clean-out-dir       Remove output directory only"
	@echo "  test                Run all workspace tests"
	@echo "  clippy              Run clippy lints"
	@echo "  check               Run full checks (all features + examples)"
	@echo "  fmt                 Format Rust files and Cargo.toml manifests"
	@echo "  help                Show this help"

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


.PHONY: check check-all-features-host check-all-features-wasm check-examples

check: check-all-features check-examples
check-all-features: check-all-features-host check-all-features-wasm

check-examples:
	RUSTFLAGS='$(RUSTFLAGS_CHECK)' cargo clippy --workspace --examples

check-all-features-host::
	$(CARGO_CHECK_HOST) --workspace --each-feature --exclude-no-default-features \
	    $(foreach c,$(CRATES_AT_LEAST_ONE_VARIANT),--exclude $(call crate_name,$c))

$(foreach c,$(CRATES_AT_LEAST_ONE_VARIANT),\
  $(eval check-all-features-host::; \
    $(CARGO_CHECK_HOST) -p $(call crate_name,$c) --feature-powerset --at-least-one-of $(call crate_features,$c)))

check-all-features-wasm::
	$(CARGO_CHECK_WASM) --workspace --each-feature --exclude-no-default-features \
	    $(foreach c,$(CRATES_AT_LEAST_ONE_VARIANT),--exclude $(call crate_name,$c)) \
	    $(addprefix --exclude ,$(CRATES_HOST_ONLY))

$(foreach c,$(CRATES_AT_LEAST_ONE_VARIANT),\
  $(eval check-all-features-wasm::; \
    $(CARGO_CHECK_WASM) -p $(call crate_name,$c) --feature-powerset --at-least-one-of $(call crate_features,$c)))
.PHONY: fmt
fmt:
	cargo fmt --all
	taplo format
