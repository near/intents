use crate::{
    Deadline, DefuseError, ExpirableNonce, Nonce, Result,
    nonce::{
        salted::{Salt, SaltedNonce, ValidSalts},
        versioned::VersionedNonce,
    },
};

struct NonceVerifier {
    salts: ValidSalts,
    deadline: Deadline,
}

impl NonceVerifier {
    pub fn new(salts: ValidSalts, deadline: Deadline) -> Self {
        Self { salts, deadline }
    }

    fn contains_salt(&self, salt: &Salt) -> bool {
        self.salts.is_valid(&salt)
    }

    pub fn valid_for_commitment(&self, nonce: Nonce) -> Result<()> {
        let versioned_nonce: VersionedNonce = nonce.into();

        match versioned_nonce {
            VersionedNonce::Legacy(_) => {
                return Ok(());
            }
            VersionedNonce::V1(SaltedNonce {
                salt,
                nonce:
                    ExpirableNonce {
                        deadline: nonce_deadline,
                        ..
                    },
            }) => {
                if !self.contains_salt(&salt) {
                    return Err(DefuseError::InvalideNonceSalt);
                }

                if self.deadline > nonce_deadline {
                    return Err(DefuseError::DeadlineGreaterThanNonce);
                }

                if nonce_deadline.has_expired() {
                    return Err(DefuseError::NonceExpired);
                }

                Ok(())
            }
        }
    }

    pub fn valid_for_clearing(&self, nonce: Nonce) -> bool {
        match VersionedNonce::try_from(nonce) {
            Ok(VersionedNonce::V1(salted)) => {
                !self.contains_salt(&salted.salt) || salted.nonce.has_expired()
            }
            Ok(VersionedNonce::Legacy(_)) => true,
            Err(_) => false,
        }
    }
}
