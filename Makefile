ROOT_DIR := $(dir $(abspath $(firstword $(MAKEFILE_LIST))))
DEFUSE_OUT_DIR ?= $(ROOT_DIR)res

.DEFAULT_GOAL := all

.PHONY: defuse \
		defuse-imt \
		poa-factory \
		poa-token \
		poa-token-no-registration \
		escrow-swap \
		global-deployer \
		multi-token-receiver-stub \
		all \
		clean-out-dir \
		clean \
		test \
		clippy

defuse defuse-imt poa-factory poa-token poa-token-no-registration escrow-swap global-deployer multi-token-receiver-stub: %: build-% 

all: \
	build-defuse \
	build-defuse-imt \
	build-poa-factory \
	build-poa-token \
	build-poa-token-no-registration \
	build-escrow-swap \
	build-global-deployer \
	build-multi-token-receiver-stub

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

# ============================================================================

build-defuse build-defuse-imt: CRATE_NAME=defuse
build-defuse-imt: CONTRACT_OUT_DIR=imt
build-defuse-imt: VARIANT=imt

# ============================================================================

build-poa-factory: CRATE_NAME=defuse-poa-factory

# ============================================================================

build-poa-token build-poa-token-no-registration: CRATE_NAME=defuse-poa-token
build-poa-token-no-registration: CONTRACT_OUT_DIR=poa-token-no-registration
build-poa-token-no-registration: VARIANT=no_registration

# ============================================================================

build-escrow-swap: CRATE_NAME=defuse-escrow-swap

# ============================================================================

build-global-deployer: CRATE_NAME=defuse-global-deployer

# ============================================================================

build-multi-token-receiver-stub: CRATE_NAME=multi-token-receiver-stub

# ============================================================================

clean-out-dir:
	rm -rf $(DEFUSE_OUT_DIR)

clean: clean-out-dir
	cargo clean

test:
	cargo test --workspace --all-targets

clippy:
	cargo clippy --workspace --all-targets --no-deps
