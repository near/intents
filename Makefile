DEFUSE_OUT_DIR ?= res

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

DEFUSE_MANIFEST_PATH := defuse/Cargo.toml
DEFUSE_FEATURES := abi,contract,imt

.PHONY: build-defuse
build-defuse:
	$(call build_non_reproducible,$(DEFUSE_MANIFEST_PATH),--features=$(DEFUSE_FEATURES))

.PHONY: build-defuse-reproducible
build-defuse-reproducible:
	$(call build_reproducible,$(DEFUSE_MANIFEST_PATH))

# ============================================================================
# Poa Factory
# ============================================================================

POA_FACTORY_MANIFEST_PATH := poa-factory/Cargo.toml
POA_FACTORY_FEATURES := contract

.PHONY: build-poa-factory
build-poa-factory:
	$(call build_non_reproducible,$(POA_FACTORY_MANIFEST_PATH),--features=$(POA_FACTORY_FEATURES))
.PHONY: build-poa-factory-reproducible
build-poa-factory-reproducible:
	$(call build_reproducible,$(POA_FACTORY_MANIFEST_PATH))

# ============================================================================
# Poa Token
# ============================================================================

POA_TOKEN_MANIFEST_PATH := poa-token/Cargo.toml
POA_TOKEN_FEATURES := contract

.PHONY: build-poa-token
build-poa-token:
	$(call build_non_reproducible,$(POA_TOKEN_MANIFEST_PATH),--features=$(POA_TOKEN_FEATURES))
.PHONY: build-poa-token-reproducible
build-poa-token-reproducible:
	$(call build_reproducible,$(POA_TOKEN_MANIFEST_PATH))

# ============================================================================
# Escrow Swap
# ============================================================================

ESCROW_SWAP_MANIFEST_PATH := escrow-swap/Cargo.toml
ESCROW_SWAP_FEATURES := abi,contract

.PHONY: build-escrow-swap
build-escrow-swap:
	$(call build_non_reproducible,$(ESCROW_SWAP_MANIFEST_PATH),--features=$(ESCROW_SWAP_FEATURES))
.PHONY: build-escrow-swap-reproducible
build-escrow-swap-reproducible:
	$(call build_reproducible,$(ESCROW_SWAP_MANIFEST_PATH))

# ============================================================================
# Global Deployer
# ============================================================================

GLOBAL_DEPLOYER_MANIFEST_PATH := global-deployer/Cargo.toml
GLOBAL_DEPLOYER_FEATURES := abi,contract

.PHONY: build-global-deployer
build-global-deployer:
	$(call build_non_reproducible,$(GLOBAL_DEPLOYER_MANIFEST_PATH),--features=$(GLOBAL_DEPLOYER_FEATURES))
.PHONY: build-global-deployer-reproducible
build-global-deployer-reproducible:
	$(call build_reproducible,$(GLOBAL_DEPLOYER_MANIFEST_PATH))

# ============================================================================
# Multi Token Receiver Stub
# ============================================================================

MULTI_TOKEN_RECEIVER_STUB_MANIFEST_PATH := tests/contracts/multi-token-receiver-stub/Cargo.toml
MULTI_TOKEN_RECEIVER_STUB_FEATURES := abi

.PHONY: build-multi-token-receiver-stub
build-multi-token-receiver-stub:
	$(call build_non_reproducible,$(MULTI_TOKEN_RECEIVER_STUB_MANIFEST_PATH), \
		--features=$(MULTI_TOKEN_RECEIVER_STUB_FEATURES))
.PHONY: build-multi-token-receiver-stub-reproducible
build-multi-token-receiver-stub-reproducible:
	$(call build_reproducible,$(MULTI_TOKEN_RECEIVER_STUB_MANIFEST_PATH))

# ============================================================================
# Utils
# ============================================================================

.PHONY: sha256
sha256:
	@for wasm in $(DEFUSE_OUT_DIR)/*.wasm; do \
		[ -f "$$wasm" ] || continue; \
		echo "Generating SHA256 for $$wasm"; \
		sha256sum $$wasm | awk '{print $$1}' > $${wasm%.wasm}.sha256; \
	done

.PHONY: clean
clean:
	cargo clean
	rm -rf $(DEFUSE_OUT_DIR)