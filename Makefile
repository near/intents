ROOT_DIR := $(dir $(abspath $(firstword $(MAKEFILE_LIST))))
DEFUSE_OUT_DIR ?= $(ROOT_DIR)res

.DEFAULT_GOAL := build-all 

.PHONY: build-defuse \
		build-defuse-imt \
		build-poa-factory \
		build-poa-token \
		build-poa-token-no-registration \
		build-escrow-swap \
		build-global-deployer \
		build-multi-token-receiver-stub \
		build-all \
		sha256 \
		clean-out-dir \
		clean \
		test

# NOTE: Build defuse with imt feature by default
build-all: \
	build-defuse-imt \
	build-poa-factory \
	build-poa-token \
	build-escrow-swap \
	build-global-deployer \
	build-multi-token-receiver-stub

METADATA_FILTER = .packages[] | select(.name == "$(CRATE_NAME)") | .metadata.near.reproducible_build
VARIANT_FILTER = .variant["$(VARIANT)"] | .container_build_command
DEFAULT_VARIANT_FILTER = .container_build_command

VAR_VALIDATION = [ -n "$(MANIFEST_PATH)" ] && [ -n "$(CRATE_NAME)" ] || { echo "MANIFEST_PATH and CRATE_NAME should be set"; exit 1; }

build-%:
ifeq ($(REPRODUCIBLE), $(filter $(REPRODUCIBLE), 1 true))

	cargo near build reproducible-wasm \
	 --manifest-path=$(MANIFEST_PATH) \
	 --out-dir=$(DEFUSE_OUT_DIR) \
	$(if $(VARIANT),--variant=$(VARIANT))

else

	@$(VAR_VALIDATION) && \
	\
	BUILD_CMD=$$(cargo metadata --format-version=1 | \
	jq -r '$(METADATA_FILTER) | \
	$(if $(VARIANT),$(VARIANT_FILTER), $(DEFAULT_VARIANT_FILTER)) \
	| join(" ")'); \
	\
	$$BUILD_CMD --manifest-path=$(MANIFEST_PATH) --out-dir=$(DEFUSE_OUT_DIR)

endif


build-defuse build-defuse-imt: CRATE_NAME=defuse
build-defuse build-defuse-imt: MANIFEST_PATH=defuse/Cargo.toml
build-defuse-imt: VARIANT=imt

# ============================================================================

build-poa-factory: CRATE_NAME=defuse-poa-factory
build-poa-factory: MANIFEST_PATH=poa-factory/Cargo.toml

build-poa-token build-poa-token-no-registration: CRATE_NAME=defuse-poa-token
build-poa-token build-poa-token-no-registration: MANIFEST_PATH=poa-token/Cargo.toml
build-poa-token-no-registration: VARIANT=no_registration

# ============================================================================

build-escrow-swap: CRATE_NAME=defuse-escrow-swap
build-escrow-swap: MANIFEST_PATH=escrow-swap/Cargo.toml

# ============================================================================

build-global-deployer: CRATE_NAME=defuse-global-deployer
build-global-deployer: MANIFEST_PATH=global-deployer/Cargo.toml

# ============================================================================

build-multi-token-receiver-stub: CRATE_NAME=multi-token-receiver-stub
build-multi-token-receiver-stub: MANIFEST_PATH=tests/contracts/multi-token-receiver-stub/Cargo.toml

# ============================================================================

sha256: $(patsubst %.wasm,%.sha256,$(wildcard $(DEFUSE_OUT_DIR)/*.wasm))
%.sha256: %.wasm
	@sha256sum $< | tee /dev/stderr | cut -d' ' -f1 > $@

clean-out-dir:
	rm -rf $(DEFUSE_OUT_DIR)

clean: clean-out-dir
	cargo clean

test:
	cargo test --workspace --all-targets

clippy:
	cargo clippy --workspace --all-targets --no-deps
