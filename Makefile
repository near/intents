ROOT_DIR := $(dir $(abspath $(firstword $(MAKEFILE_LIST))))
DEFUSE_OUT_DIR ?= $(ROOT_DIR)res
CARGO_EXTRA_FLAGS?=

CARGO_NEAR := cargo near build
NON_REPRO_FLAGS := non-reproducible-wasm --locked --no-embed-abi --out-dir $(DEFUSE_OUT_DIR)
REPRO_FLAGS := reproducible-wasm --out-dir $(DEFUSE_OUT_DIR)

define build_non_reproducible
	$(CARGO_NEAR) $(NON_REPRO_FLAGS) --manifest-path $(1) $(2)
endef

define build_reproducible
	$(CARGO_NEAR) $(REPRO_FLAGS) --manifest-path $(1)
endef

.PHONY: all
all: \
	build-defuse \
	build-poa-factory \
	build-poa-token \
	build-escrow-swap \
	build-global-deployer \
	build-multi-token-receiver-stub

.PHONY: all-reproducible
all-reproducible: \
	build-defuse-reproducible \
	build-poa-factory-reproducible \
	build-poa-token-reproducible \
	build-escrow-swap-reproducible \
	build-global-deployer-reproducible \
	build-multi-token-receiver-stub-reproducible

# ============================================================================
# Defuse
# ============================================================================

DEFUSE_MANIFEST_PATH := $(ROOT_DIR)defuse/Cargo.toml
DEFUSE_FEATURES := --features=abi,contract,imt
DEFUSE_FLAGS ?= $(if $(CARGO_EXTRA_FLAGS),$(CARGO_EXTRA_FLAGS),$(DEFUSE_FEATURES))

.PHONY: build-defuse
build-defuse:
	$(call build_non_reproducible,$(DEFUSE_MANIFEST_PATH),$(DEFUSE_FLAGS))

.PHONY: build-defuse-reproducible
build-defuse-reproducible:
	$(call build_reproducible,$(DEFUSE_MANIFEST_PATH))

# ============================================================================
# Poa Factory
# ============================================================================

POA_FACTORY_MANIFEST_PATH := $(ROOT_DIR)poa-factory/Cargo.toml
POA_FACTORY_FEATURES := --features=contract
POA_FACTORY_FLAGS ?= $(if $(CARGO_EXTRA_FLAGS),$(CARGO_EXTRA_FLAGS),$(POA_FACTORY_FEATURES))

.PHONY: build-poa-factory
build-poa-factory:
	$(call build_non_reproducible,$(POA_FACTORY_MANIFEST_PATH),$(POA_FACTORY_FLAGS))
	
.PHONY: build-poa-factory-reproducible
build-poa-factory-reproducible:
	$(call build_reproducible,$(POA_FACTORY_MANIFEST_PATH))

# ============================================================================
# Poa Token
# ============================================================================

POA_TOKEN_MANIFEST_PATH := $(ROOT_DIR)poa-token/Cargo.toml
POA_TOKEN_FEATURES := --features=contract
POA_TOKEN_FLAGS ?= $(if $(CARGO_EXTRA_FLAGS),$(CARGO_EXTRA_FLAGS),$(POA_TOKEN_FEATURES))

.PHONY: build-poa-token
build-poa-token:
	$(call build_non_reproducible,$(POA_TOKEN_MANIFEST_PATH),$(POA_TOKEN_FLAGS))
.PHONY: build-poa-token-reproducible
build-poa-token-reproducible:
	$(call build_reproducible,$(POA_TOKEN_MANIFEST_PATH))

# ============================================================================
# Escrow Swap
# ============================================================================

ESCROW_SWAP_MANIFEST_PATH := $(ROOT_DIR)escrow-swap/Cargo.toml
ESCROW_SWAP_FEATURES := --features=abi,contract
ESCROW_SWAP_FLAGS ?= $(if $(CARGO_EXTRA_FLAGS),$(CARGO_EXTRA_FLAGS),$(ESCROW_SWAP_FEATURES))


.PHONY: build-escrow-swap
build-escrow-swap:
	$(call build_non_reproducible,$(ESCROW_SWAP_MANIFEST_PATH),$(ESCROW_SWAP_FLAGS))
.PHONY: build-escrow-swap-reproducible
build-escrow-swap-reproducible:
	$(call build_reproducible,$(ESCROW_SWAP_MANIFEST_PATH))

# ============================================================================
# Global Deployer
# ============================================================================

GLOBAL_DEPLOYER_MANIFEST_PATH := $(ROOT_DIR)global-deployer/Cargo.toml
GLOBAL_DEPLOYER_FEATURES := --features=abi,contract
GLOBAL_DEPLOYER_FLAGS ?= $(if $(CARGO_EXTRA_FLAGS),$(CARGO_EXTRA_FLAGS),$(GLOBAL_DEPLOYER_FEATURES))

.PHONY: build-global-deployer
build-global-deployer:
	$(call build_non_reproducible,$(GLOBAL_DEPLOYER_MANIFEST_PATH),$(GLOBAL_DEPLOYER_FLAGS))
.PHONY: build-global-deployer-reproducible
build-global-deployer-reproducible:
	$(call build_reproducible,$(GLOBAL_DEPLOYER_MANIFEST_PATH))

# ============================================================================
# Multi Token Receiver Stub
# ============================================================================

MULTI_TOKEN_RECEIVER_STUB_MANIFEST_PATH := $(ROOT_DIR)tests/contracts/multi-token-receiver-stub/Cargo.toml
MULTI_TOKEN_RECEIVER_STUB_FEATURES := --features=abi
MULTI_TOKEN_RECEIVER_STUB_FLAGS ?= $(if $(CARGO_EXTRA_FLAGS),$(CARGO_EXTRA_FLAGS),$(MULTI_TOKEN_RECEIVER_STUB_FEATURES))

.PHONY: build-multi-token-receiver-stub
build-multi-token-receiver-stub:
	$(call build_non_reproducible,$(MULTI_TOKEN_RECEIVER_STUB_MANIFEST_PATH),$(MULTI_TOKEN_RECEIVER_STUB_FLAGS))
.PHONY: build-multi-token-receiver-stub-reproducible
build-multi-token-receiver-stub-reproducible:
	$(call build_reproducible,$(MULTI_TOKEN_RECEIVER_STUB_MANIFEST_PATH))

# ============================================================================
# Utils
# ============================================================================

.PHONY: sha256
sha256: $(patsubst %.wasm,%.sha256,$(wildcard $(DEFUSE_OUT_DIR)/*.wasm))
%.sha256: %.wasm
	@sha256sum $< | tee /dev/stderr | cut -d' ' -f1 > $@

.PHONY: clean-out-dir
clean-out-dir:
	rm -rf $(DEFUSE_OUT_DIR)

.PHONY: clean
clean: clean-out-dir
	cargo clean

.PHONY: clippy
clippy:
	cargo clippy --workspace --all-targets --no-deps

# ============================================================================
# Tests
# ============================================================================

.PHONY: test
test:
	cargo test --workspace --all-targets
