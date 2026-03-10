ROOT_DIR := $(dir $(abspath $(firstword $(MAKEFILE_LIST))))
DEFUSE_OUT_DIR ?= $(ROOT_DIR)res

.DEFAULT_GOAL := all

# ============================================================================

CONTRACTS += defuse defuse-far
build-defuse build-defuse-far: CRATE_NAME=defuse
build-defuse-far: CONTRACT_OUT_DIR=far
build-defuse-far: VARIANT=far

# ============================================================================

CONTRACTS += poa-factory
build-poa-factory: CRATE_NAME=defuse-poa-factory

# ============================================================================

CONTRACTS += poa-token poa-token-no-registration
build-poa-token build-poa-token-no-registration: CRATE_NAME=defuse-poa-token
build-poa-token-no-registration: CONTRACT_OUT_DIR=poa-token-no-registration
build-poa-token-no-registration: VARIANT=no_registration

# ============================================================================

CONTRACTS += escrow-swap
build-escrow-swap: CRATE_NAME=defuse-escrow-swap

# ============================================================================

CONTRACTS += global-deployer
build-global-deployer: CRATE_NAME=defuse-global-deployer

# ============================================================================

CONTRACTS += multi-token-receiver-stub
build-multi-token-receiver-stub: CRATE_NAME=multi-token-receiver-stub

# ============================================================================

CONTRACTS += escrow-proxy
build-escrow-proxy: CRATE_NAME=defuse-escrow-proxy

# ============================================================================

CONTRACTS += oneshot
build-oneshot: CRATE_NAME=defuse-oneshot-condvar

# ============================================================================

.PHONY: all
all: $(CONTRACTS)

.PHONY: $(CONTRACTS)
$(CONTRACTS): %: build-%

CARGO_METADATA = cargo metadata --format-version=1 | jq -r
CRATE_FILTER = .packages[] | select(.name == "$(CRATE_NAME)")

MANIFEST_PATH = $(shell $(CARGO_METADATA) '$(CRATE_FILTER) | .manifest_path')

ifneq (,$(filter $(shell printf '%s' $(REPRODUCIBLE) | tr '[:upper:]' '[:lower:]'), 1 true on))
BUILD_CMD = cargo near build reproducible-wasm $(if $(VARIANT),--variant=$(VARIANT))
else
BUILD_CMD = $(shell $(CARGO_METADATA) \
			'$(CRATE_FILTER) | .metadata.near.reproducible_build | \
			$(if $(VARIANT),.variant["$(VARIANT)"] |,) \
			.container_build_command | join(" ")')
endif

build-%:
	$(if $(CRATE_NAME),,$(error CRATE_NAME is not defined))
	$(BUILD_CMD) \
		--manifest-path=$(MANIFEST_PATH) \
		--out-dir="$(DEFUSE_OUT_DIR)/$(CONTRACT_OUT_DIR)"

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
