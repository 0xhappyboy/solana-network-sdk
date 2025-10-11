use solana_sdk::{signature::Signature, signer::Signer};

use crate::wallet::Wallet;

pub struct Message;
impl Message {
    /// sign message
    /// # params
    /// * w wallet object
    /// * message byte array of the message
    /// ```rust
    /// let w = Wallet::from_private_key_64("private key");
    /// let s = Message::sign_message(w, "message".as_bytes());
    /// ```
    pub fn sign_message(w: Wallet, message: &[u8]) -> Signature {
        w.keypair.unwrap().sign_message(message)
    }
    /// verify message
    /// # params
    /// * signature signature object
    /// * w wallet object
    /// * message byte array of the message
    /// ```rust
    /// let w = Wallet::from_private_key_64("private key");
    /// let s = Message::sign_message(w, "message".as_bytes());
    /// let bool = Message::verify_message(s, w, "message".as_bytes());
    /// ```
    pub fn verify_message(signature: Signature, w: Wallet, message: &[u8]) -> bool {
        signature.verify(w.keypair.unwrap().pubkey().as_ref(), message)
    }
}
