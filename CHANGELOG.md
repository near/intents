# Changelog

All notable changes to this project will be documented in this file.

## [0.3.0]

### Added
- 4 bytes salts added to state for security purposes
- SaltManager role to manage salts rotation functionality
- GarbageCollector role to cleanup expired/invalid nonces
- Expirable nonces: contain expiration deadline in nanoseconds
- Salted nonces: contain salt part as validity identifier
- Cleanup functionality for nonces to preserve storage: all expired nonces or nonces with invalid salt can be cleared. Legacy nonces can't be cleared before a complete prohibition on its usage.

### Changed
- Contract storage versioning
- State versioning
- Nonce versioning
- Updated nonce format: The new version must contain the salt part + expiration date to determine if it is valid, but legacy nonces are still supported
- Optimized nonce storage by adding hashers into maps
- Intent simulation output returns all nonces committed by transaction

