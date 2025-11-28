# Verifier smart contracts
Important concepts

## Salt

Unique 4 byte value which is stored and managed under the contract state for security purposes.  
Verifier's salt registry can contain several valid salts at the same time. Admins can disable or rotate salts in extreme cases, but there will always be at least one valid salt.  
The most recent salt can be obtained using the [current_salt](https://github.com/near/intents/blob/main/defuse/src/salts.rs#L20) view method which returns hexadecimal string. Also in order to check the validity of the used salt the [is_valid_salt](https://github.com/near/intents/blob/main/defuse/src/salts.rs#L17) method can be used

## Nonces

In order to prevent security attacks it is required for each intent to be accompanied with unique nonce.  
Verifier smart contract supports [unordered](https://docs.uniswap.org/contracts/permit2/reference/signature-transfer#nonce-schema) nonces schema. 
Currently, each individual nonce is considered as valid in the following formats:
- Legacy nonce
- Versioned nonce

---

### Legacy nonce
A unique 256-bit randomly generated number that was initially used without additional checks.  
**<span style="color: red;">But in the near future, will be prohibited and the only valid nonce schema will be a versioned nonce.</span>**

### Versioned nonce
The initial idea behind its creation was to enabling new nonce formats which can contain metadata that could identify it according to certain parameters and determine its validity. Nonces that fall under the criteria of invalid (which are individual for each version) can be removed from the contract by the relevant delegates.

Versioned nonce also consists of 32 bytes but has a specific structure:

1. Prefix (5 bytes): 
To distinguish between legacy and versioned nonces we use a specific prefix individual for each version consisting of 2 parts:
- Magic prefix for all versioned nonces: first 4 bytes of `sha256(<versioned_nonce>)` which is `"5628f6c6"` as hex string.
- 1 byte nonce version index.

2. Body (27 bytes): specific for each nonce version.

Supported nonce versions:
- V1:  
**Body**: salt (4 bytes) + deadline in nanoseconds (8 bytes) + nonce (15 random bytes).  
**Validity**: A nonce is considered valid if its salt is one of the valid salts in the contract salt registry and if **the deadline of the intent does not exceed nonce deadline** (which cannot be less than the current time). Accordingly, a nonce becomes invalid and can be cleaned up if the deadline passes or if its salt is no longer considered as valid.  
**NOTE**: The deadline for nonce may be greater than the deadline for intent — it is important to take this into account in order to allow for the possibility of invalidating nonces in case of intent cancellation.

---

### Example:  

Data:
- Deadline: 2280047743 sec. 
- Salt: `252812b3`. 
- Random bytes: `027015ec13dc11864973138fe6812f`. 

Encoded bytes (hex):  
56 28 f6 c6 | 00 | 25 28 12 b3 | 00 36 72 90 57 5b a4 1f | 02 70 15 ec 13 dc 11 86 49 73 13 8f e6 81 2f

| Field        | Bytes                                          | Size | Meaning                                |
| ------------ | ---------------------------------------------- | ---- | -------------------------------------- |
| **Prefix**   | `56 28 f6 c6`                                  | 4    | Magic prefix                           |
| **Version**  | `00`                                           | 1    | Version `0`                            |
| **Salt**     | `25 28 12 b3`                                  | 4    | Salt bytes                             |
| **Deadline** | `00 36 72 90 57 5b a4 1f`                      | 8    | Little-endian-encoded timestamp        |
| **Nonce**    | `02 70 15 ec 13 dc 11 86 49 73 13 8f e6 81 2f` | 15   | Random nonce bytes                     |


Result as Base64 string:   
Vij2xgAlKBKzADZykFdbpB8CcBXsE9wRhklzE4/mgS8=