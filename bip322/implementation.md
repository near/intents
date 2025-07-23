
### Dependencies to Remove
- `bip322` crate - replace with native implementation
- `digest` crate - use NEAR SDK functions instead
- Minimize `bitcoin` crate features to bare minimum

### Key Design Decisions

1. **Bitcoin Mainnet Only**: Hardcode all mainnet parameters, remove network abstraction
2. **NEAR SDK Crypto**: Use only NEAR host functions for all cryptographic operations
3. **Custom Implementation**: Implement BIP-322 logic from scratch using minimal dependencies
4. **Breaking Changes**: Complete rewrite of public API if needed for optimization
5. **Gas Optimized**: Every operation optimized for NEAR gas costs

## Implementation Timeline

### Phase 1: Foundation & Gas Benchmarking (1-2 days)
- Remove external dependencies
- Set up basic structure with NEAR SDK primitives
- Implement core hash and signature functions using NEAR SDK
- **Early gas benchmarking** with NEAR SDK crypto functions to validate feasibility

### Phase 2: MVP - Simple Address Types (3-4 days)  
- Implement BIP-322 transaction creation natively
- **P2PKH support**: Legacy addresses (starting with '1')
- **P2WPKH support**: Bech32 addresses (starting with 'bc1q')
- Basic signature verification pipeline for simple address types
- Public key recovery with fallback error handling

### Phase 3: MVP Integration & Validation (2-3 days)
- Complete Payload/SignedPayload implementation for P2PKH/P2WPKH
- Integration testing with existing intents system
- Performance optimization and gas profiling
- Test against BIP-322 specification for simple address types
- **Compatibility validation** with popular Bitcoin wallets

### Phase 4: Complex Address Types Extension (3-4 days)
- **P2SH support**: Script hash addresses (starting with '3') 
- **P2WSH support**: Complex script witness addresses
- Handle redeem script reconstruction for P2SH
- Extended signature verification for complex types
- Comprehensive fallback strategies for edge cases

### Phase 5: Final Validation & Optimization (1-2 days)
- Complete BIP-322 specification compliance testing
- Final performance tuning and gas optimization
- Integration testing for all address types
- Bitcoin ecosystem compatibility validation

## Technical Architecture

### Core Components

1. **Address Handler**: Custom Bitcoin address parsing and validation (mainnet only)
2. **Transaction Builder**: Native BIP-322 transaction creation using NEAR SDK
3. **Signature Verifier**: Complete verification pipeline using NEAR host functions
4. **Hash Calculator**: All hashing operations using `near_sdk::env::sha256_array()`
5. **Public Key Recovery**: Using `near_sdk::env::ecrecover()` exclusively

### NEAR SDK Integration Points

- `near_sdk::env::sha256_array()` for double SHA-256 operations
- `near_sdk::env::sha256()` for message hashing
- `near_sdk::env::ripemd160_array()` for RIPEMD-160 hash computation
- `near_sdk::env::ecrecover()` for public key recovery
- Existing defuse-crypto types for public key representation
- NEAR gas optimization patterns

### Supported Address Types (Bitcoin Mainnet Only)

**MVP Implementation (Phase 2-3):**
- **P2PKH**: Pay to Public Key Hash (legacy addresses starting with '1')
- **P2WPKH**: Pay to Witness Public Key Hash (bech32 addresses starting with 'bc1q')

**Extended Implementation (Phase 4):**
- **P2SH**: Pay to Script Hash (addresses starting with '3')
- **P2WSH**: Pay to Witness Script Hash (bech32 addresses for complex scripts)

## Success Criteria

**MVP (Phases 1-3):**
1. **Zero external crypto dependencies** - all operations use NEAR SDK
2. **Minimal external dependencies** - only essential bitcoin types
3. **P2PKH/P2WPKH BIP-322 compliance** - passes relevant test vectors for simple address types
4. **Gas feasibility validated** - early benchmarking confirms viability
5. **Basic intents integration** - works with existing smart contract system

**Extended (Phases 4-5):**
6. **Full BIP-322 compliance** - passes all relevant test vectors including complex address types
7. **Gas optimized** - comparable or better performance than existing implementations
8. **Wallet compatibility** - works with popular Bitcoin wallets implementing BIP-322
9. **Robust error handling** - comprehensive fallback strategies for edge cases

## Testing Strategy

### Unit Tests
- Individual component testing with known test vectors
- Hash calculation validation against BIP-322 specification
- Address parsing and validation tests
- Signature verification test cases

### Integration Tests
- End-to-end BIP-322 message verification
- Integration with existing Payload/SignedPayload traits
- Gas consumption benchmarking
- Compatibility with intents execution pipeline

### Test Vectors
- Official BIP-322 test vectors
- Bitcoin Core test cases
- Custom test cases for edge conditions
- Performance benchmarks against external implementations

## Migration Strategy

Since breaking changes are allowed:

1. **Complete Rewrite**: Replace existing implementation entirely
2. **New API Design**: Optimize API for NEAR SDK usage patterns
3. **Remove Compatibility Layer**: No need to maintain backward compatibility
4. **Direct Integration**: Direct integration with intents system from the start

## Performance Targets

- **Gas Usage**: Comparable to or better than other signature verification methods in the intents system
- **Execution Time**: Sub-second verification for typical use cases
- **Memory Usage**: Minimal memory allocation, leverage NEAR SDK efficiently
- **Binary Size**: Minimal impact on contract size due to reduced dependencies

## Risk Mitigation

- **Gas Cost Overruns**: Early benchmarking in Phase 1 to validate NEAR SDK crypto feasibility
- **Public Key Recovery Failures**: Implement comprehensive fallback strategies for non-recoverable signatures
- **P2SH Complexity**: Limit initial scope to P2PKH/P2WPKH, add P2SH only after MVP validation
- **Specification Compliance**: Thorough testing against BIP-322 specification with incremental validation
- **Bitcoin Compatibility**: Validation against Bitcoin Core implementations and popular wallets
- **Performance Regression**: Continuous benchmarking during development
- **Integration Issues**: Early integration testing with existing intents components

## Implementation Strategy Summary

This updated implementation plan provides a **risk-mitigated roadmap** for creating a highly optimized, NEAR-native BIP-322 implementation:

**Phase 1-3 (MVP)**: Focus on P2PKH/P2WPKH address types with early gas validation and wallet compatibility testing. This approach reduces complexity while validating the core approach.

**Phase 4-5 (Extended)**: Add complex address types (P2SH/P2WSH) only after MVP success, with comprehensive fallback strategies for edge cases.

The phased approach allows for early validation of gas costs and technical feasibility before tackling the more complex aspects of BIP-322, while still delivering a complete implementation that minimizes external dependencies and maximizes performance within the intents ecosystem.
