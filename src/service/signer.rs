use async_trait::async_trait;
use coin_shuffle_core::node::signer;
use ethers::{
    signers::{LocalWallet, Signer as EthersSigner, WalletError},
    types::Signature,
};

#[derive(Debug, Clone)]
pub struct DirectSigner {
    wallet: LocalWallet,
}

impl DirectSigner {
    pub fn new(wallet: LocalWallet) -> Self {
        Self { wallet }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl signer::Signer for DirectSigner {
    type Error = WalletError;

    async fn sign_message<S: Send + Sync + AsRef<[u8]>>(
        &self,
        message: S,
    ) -> Result<Signature, Self::Error> {
        Ok(self.wallet.sign_message(message).await?)
    }
}
