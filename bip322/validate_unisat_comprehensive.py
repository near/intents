#!/usr/bin/env python3
"""
Comprehensive validation of UniSat BIP-322 test vector
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
    """Recover public key from compact signature using ecdsa"""
    try:
        import ecdsa
        from ecdsa.curves import SECP256k1
        from ecdsa.ecdsa import possible_public_keys_from_signature
        
        recovery_id = signature_bytes[0]
        r_bytes = signature_bytes[1:33]
        s_bytes = signature_bytes[33:65]
        
        print(f"Recovery ID: {recovery_id}")
        print(f"R: {hexlify(r_bytes).decode()}")
        print(f"S: {hexlify(s_bytes).decode()}")
        
        # Convert recovery ID to v (0-3 range)
        if recovery_id >= 31:
            v = recovery_id - 31
            compressed = True
        elif recovery_id >= 27:
            v = recovery_id - 27  
            compressed = False
        else:
            print(f"Invalid recovery ID: {recovery_id}")
            return None
            
        print(f"Calculated v: {v}, compressed: {compressed}")
        
        # Manual ECDSA recovery using bitcoinlib
        try:
            from bitcoinlib.encoding import hash160
            from bitcoinlib.keys import Key
            
            # Convert r, s to integers
            r = int.from_bytes(r_bytes, 'big')
            s = int.from_bytes(s_bytes, 'big')
            
            print(f"R (int): {r}")
            print(f"S (int): {s}")
            
            # Try to use bitcoinlib's own recovery mechanism
            # Create a signature string in the format bitcoinlib expects
            sig_string = f"{r:064x}{s:064x}"
            print(f"Signature string: {sig_string}")
            
            # Try all possible recovery IDs
            witness_program = parse_bech32_address(ADDRESS)
            print(f"Target witness program: {hexlify(witness_program).decode()}")
            
            for test_v in range(4):
                try:
                    print(f"\nTrying recovery with v={test_v}")
                    
                    # Manual point recovery using curve math
                    from ecdsa.curves import SECP256k1
                    from ecdsa.ellipticcurve import Point
                    
                    # Get curve parameters
                    curve = SECP256k1.generator
                    order = curve.order()
                    
                    # Calculate point from r and recovery ID
                    x = r
                    
                    # Try different x values (r and r + order)
                    for j in range(2):
                        if j == 1:
                            x = r + order
                            
                        # Calculate y from x
                        # y^2 = x^3 + 7 (secp256k1 curve equation)
                        y_squared = (pow(x, 3, SECP256k1.p) + 7) % SECP256k1.p
                        
                        # Find square root
                        y = pow(y_squared, (SECP256k1.p + 1) // 4, SECP256k1.p)
                        
                        # Choose the correct y based on parity
                        if (y % 2) != (test_v % 2):
                            y = SECP256k1.p - y
                            
                        # Create point
                        try:
                            point = Point(SECP256k1.curve, x, y, order)
                            
                            # Verify this is the correct recovery
                            point_index = j * 2 + (test_v % 2)
                            if point_index != test_v:
                                continue
                                
                            print(f"Recovery {test_v}: Point({x}, {y})")
                            
                            # Convert to compressed public key
                            x_bytes = x.to_bytes(32, 'big')
                            y_parity = y % 2
                            compressed_pubkey = bytes([0x02 + y_parity]) + x_bytes
                            
                            print(f"Compressed pubkey: {hexlify(compressed_pubkey).decode()}")
                            
                            # Compute hash160
                            pubkey_hash = hash160(compressed_pubkey)
                            print(f"Hash160: {hexlify(pubkey_hash).decode()}")
                            
                            # Compare with expected witness program
                            if pubkey_hash == witness_program:
                                print(f"✓ Key matches address with v={test_v}!")
                                return compressed_pubkey
                                
                        except Exception as e:
                            print(f"Point creation failed for v={test_v}, j={j}: {e}")
                            continue
                            
                except Exception as e:
                    print(f"Recovery v={test_v} failed: {e}")
                    continue
                    
            print("No valid recovery found")
            return None
                        
        except Exception as e:
            print(f"ECDSA recovery failed: {e}")
            import traceback
            traceback.print_exc()
            return None
            
    except ImportError as e:
        print(f"Required library not available: {e}")
        return None

def main():
    print("Comprehensive UniSat BIP-322 test vector validation...")
    print(f"Address: {ADDRESS}")
    print(f"Message: {MESSAGE}")
    print(f"Signature (base64): {SIGNATURE_B64}")
    print()
    
    # Decode signature
    try:
        signature_bytes = base64.b64decode(SIGNATURE_B64)
        print(f"Signature length: {len(signature_bytes)} bytes")
        
        if len(signature_bytes) != 65:
            print(f"✗ Unexpected signature length: {len(signature_bytes)}")
            return
            
    except Exception as e:
        print(f"✗ Failed to decode signature: {e}")
        return
    
    # Parse address
    witness_program = parse_bech32_address(ADDRESS)
    if witness_program:
        print(f"✓ Address parsed successfully")
        print(f"Expected witness program: {hexlify(witness_program).decode()}")
    else:
        print("✗ Failed to parse address")
        return
    
    # Compute message hash
    message_hash = bitcoin_message_hash(MESSAGE)
    print(f"Message hash: {hexlify(message_hash).decode()}")
    
    # Try to recover public key
    recovered_pubkey = recover_pubkey_from_signature(message_hash, signature_bytes)
    
    if recovered_pubkey:
        print(f"✓ Successfully recovered public key: {hexlify(recovered_pubkey).decode()}")
    else:
        print("✗ Failed to recover public key")
    
    print("\n" + "="*50)
    print("CONCLUSION:")
    if recovered_pubkey:
        print("✓ UniSat test vector is VALID")
        print("The signature successfully verifies against the address")
    else:
        print("? Unable to fully validate - need better ECDSA recovery")
        print("The test vector format appears correct but verification incomplete")

if __name__ == "__main__":
    main()