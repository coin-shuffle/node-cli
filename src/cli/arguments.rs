use clap::Args;

#[derive(Args, Debug, Clone)]
pub struct GetUTXOs {
    /// The Ethereum RPC URL
    #[clap(long, value_name = "Ethereum RPC URL")]
    pub rpc_url: String,

    /// The corresponding HEX encoded UTXO contract address
    #[clap(long, value_name = "UTXO Contract address")]
    pub utxo_address: String,

    /// The Ethereum address
    #[clap(long, value_name = "Ethereum address")]
    pub address: String,
}

#[derive(Args, Debug, Clone)]
pub struct Shuffle {
    /// The UTXO id on the corresponding UTXO contract
    #[clap(long, value_name = "UTXO ID")]
    pub utxo_id: String,

    /// The Shuffle-service URL
    #[clap(long, value_name = "Service URL")]
    pub service_url: String,

    /// The Ethereum RPC URL
    #[clap(long, value_name = "Ethereum RPC URL")]
    pub rpc_url: String,

    /// The path for file with the HEX encoded ECDSA private key 0x...
    #[clap(long, value_name = "ECDSA Private key")]
    pub ecdsa_priv_path: String,

    /// The path for file with the PEM PKCS#8 encoded RSA Private key
    #[clap(long, value_name = "RSA Private key")]
    pub rsa_priv_path: String,

    /// The corresponding HEX encoded UTXO contract address
    #[clap(long, value_name = "UTXO Contract address")]
    pub utxo_address: String,

    /// The coin shuffle output address
    #[clap(long, value_name = "Output address")]
    pub output_address: String,
}
