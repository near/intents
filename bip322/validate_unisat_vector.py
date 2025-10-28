#!/usr/bin/env python3
"""
Validate UniSat BIP-322 test vector using external Bitcoin libraries
"""

import base64
import hashlib
from binascii import hexlify, unhexlify

# Test vector data
ADDRESS = "bc1qyt6gau643sm52hvej4n4qr34h3878ahs209s27"
MESSAGE = '{"signer_id":"alice.near","verifying_contract":"defuse.near","deadline":"Never","nonce":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=","test":"value"}'
SIGNATURE_B64 = "H6Gjb7ArwmAtbS7urzjT1IS+GfGLhz5XgSvu2c863K0+RcxgOFDoD7Uo+Z44CK7NcCLY1tc9eeudsYlM2zCNYDU="

def parse_bech32_address(address):
    """Parse bech32 address to get witness program"""
    # Simple bech32 parsing (for P2WPKH)
    import bech32
    
    try:
        hrp, data = bech32.bech32_decode(address)
        if hrp == 'bc' and data:
            # Convert from 5-bit to 8-bit
            decoded = bech32.convertbits(data[1:], 5, 8, False)
            if decoded and len(decoded) == 20:
                return bytes(decoded)
    except:
        pass
    return None

def bitcoin_message_hash(message):
    """Compute Bitcoin message hash (double SHA256 with prefix)"""
    prefix = b"Bitcoin Signed Message:\n"
    message_bytes = message.encode('utf-8')
    
    # Create varint length encoding
    msg_len = len(message_bytes)
    if msg_len < 253:
        len_bytes = bytes([msg_len])
    elif msg_len <= 0xFFFF:
        len_bytes = bytes([0xFD]) + msg_len.to_bytes(2, 'little')
    elif msg_len <= 0xFFFFFFFF:
        len_bytes = bytes([0xFE]) + msg_len.to_bytes(4, 'little')
    else:
        len_bytes = bytes([0xFF]) + msg_len.to_bytes(8, 'little')
    
    # Double SHA256 hash
    full_message = prefix + len_bytes + message_bytes
    hash1 = hashlib.sha256(full_message).digest()
    hash2 = hashlib.sha256(hash1).digest()
    
    return hash2

def recover_pubkey_from_signature(message_hash, signature_bytes):
    """Try to recover public key from compact signature"""
    try:
        import ecdsa
        from ecdsa.curves import SECP256k1
        from ecdsa.ellipticcurve import Point
        
        recovery_id = signature_bytes[0]
        r_bytes = signature_bytes[1:33]
        s_bytes = signature_bytes[33:65]
        
        print(f"Recovery ID: {recovery_id}")
        print(f"R: {hexlify(r_bytes).decode()}")
        print(f"S: {hexlify(s_bytes).decode()}")
        
        # Try different recovery approaches
        for test_recovery_id in [recovery_id, recovery_id - 4, recovery_id + 4]:
            if test_recovery_id < 0 or test_recovery_id > 255:
                continue
                
            try:
                # Calculate v for ECDSA recovery (0-3 range)
                if test_recovery_id >= 31:
                    v = test_recovery_id - 31
                elif test_recovery_id >= 27:
                    v = test_recovery_id - 27
                else:
                    v = test_recovery_id
                
                if v < 0 or v > 3:
                    continue
                
                print(f"Trying recovery ID {test_recovery_id} -> v={v}")
                
                # This is a simplified recovery attempt
                # In practice, you'd use a proper ECDSA library
                
            except Exception as e:
                print(f"Recovery failed for ID {test_recovery_id}: {e}")
                continue
                
        return None
        
    except ImportError:
        print("ecdsa library not available")
        return None

def validate_address_pubkey(pubkey_bytes, address):
    """Validate that public key matches the address"""
    try:
        import hashlib
        
        # For P2WPKH, we need HASH160 of compressed pubkey
        if len(pubkey_bytes) == 64:
            # Uncompressed, need to compress
            x = pubkey_bytes[:32]
            y = pubkey_bytes[32:]
            
            # Determine compression prefix
            y_int = int.from_bytes(y, 'big')
            prefix = 0x02 if y_int % 2 == 0 else 0x03
            compressed = bytes([prefix]) + x
        else:
            compressed = pubkey_bytes
            
        # HASH160 = RIPEMD160(SHA256(pubkey))
        sha256_hash = hashlib.sha256(compressed).digest()
        # We'd need ripemd160 library for full validation
        
        return True  # Placeholder
        
    except Exception as e:
        print(f"Address validation failed: {e}")
        return False

def main():
    print("Validating UniSat BIP-322 test vector...")
    print(f"Address: {ADDRESS}")
    print(f"Message: {MESSAGE}")
    print(f"Signature (base64): {SIGNATURE_B64}")
    print()
    
    # Decode signature
    try:
        signature_bytes = base64.b64decode(SIGNATURE_B64)
        print(f"Signature length: {len(signature_bytes)} bytes")
        
        if len(signature_bytes) == 65:
            print("✓ Signature is 65 bytes (compact format)")
        else:
            print(f"✗ Unexpected signature length: {len(signature_bytes)}")
            return
            
    except Exception as e:
        print(f"✗ Failed to decode signature: {e}")
        return
    
    # Parse address
    witness_program = parse_bech32_address(ADDRESS)
    if witness_program:
        print(f"✓ Address parsed successfully")
        print(f"Witness program (20 bytes): {hexlify(witness_program).decode()}")
    else:
        print("✗ Failed to parse address")
        return
    
    # Compute message hash
    message_hash = bitcoin_message_hash(MESSAGE)
    print(f"Message hash: {hexlify(message_hash).decode()}")
    
    # Try to recover public key
    recovered_pubkey = recover_pubkey_from_signature(message_hash, signature_bytes)
    
    # Check if we have the necessary libraries
    try:
        import bech32
        print("✓ bech32 library available")
    except ImportError:
        print("✗ bech32 library not available - run: pip install bech32")
    
    try:
        import ecdsa
        print("✓ ecdsa library available")
    except ImportError:
        print("✗ ecdsa library not available - run: pip install ecdsa")
    
    # Summary
    print("\nSummary:")
    print("This test vector appears to be a P2WPKH compact signature.")
    print("The signature format is correct (65 bytes).")
    print("Further validation requires proper ECDSA recovery implementation.")

if __name__ == "__main__":
    main()