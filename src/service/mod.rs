use coin_shuffle_contracts_bindings::utxo::Connector;
use coin_shuffle_core::node::room::Room;
use coin_shuffle_core::node::storage::RoomMemoryStorage;
use coin_shuffle_core::node::Node;
use coin_shuffle_protos::v1::shuffle_service_client::ShuffleServiceClient;
use coin_shuffle_protos::v1::{ConnectShuffleRoomRequest, JoinShuffleRoomRequest};
use ethers::providers::{Http, Provider};
use ethers::signers::{LocalWallet, Signer};
use ethers::types::U256;
use eyre::{Context, ContextCompat, Result};
use rsa::pkcs8::DecodePrivateKey;
use rsa::RsaPrivateKey;
use std::fs::read_to_string;
use std::str::FromStr;
use std::time::SystemTime;
use tonic::transport::Channel;

#[derive(Debug, Clone)]
pub struct Service {
    node: Node<RoomMemoryStorage, Connector<Provider<Http>>>,
    grpc_service: ShuffleServiceClient<Channel>,
    room: Option<Room>,
    jwt: Option<String>,
}

impl Service {
    pub fn new(
        rpc_url: String,
        utxo_address: String,
        grpc_service: ShuffleServiceClient<Channel>,
    ) -> Result<Self> {
        Ok(Self {
            node: Node::new(
                RoomMemoryStorage::new(),
                Connector::from_raw(rpc_url, utxo_address)
                    .context("failed to init connector from raw")?,
            )
            .into(),
            grpc_service,
            room: None,
            jwt: None,
        })
    }

    pub async fn init_shuffle_room(
        &mut self,
        utxo_id: U256,
        output_address: String,
        rsa_priv_path: String,
        ecdsa_priv_path: String,
    ) -> Result<Self> {
        self.room = Some(
            self.node
                .init_room(
                    utxo_id,
                    output_address.as_bytes().to_vec(),
                    RsaPrivateKey::from_pkcs8_pem(
                        read_to_string(rsa_priv_path)
                            .context("failed to read rsa priv key from file")?
                            .as_str(),
                    )?,
                    LocalWallet::from_str(
                        read_to_string(ecdsa_priv_path)
                            .context("failed to read ecdsa priv key from file")?
                            .as_str(),
                    )
                    .context("failed to parse ecdsa priv key")?,
                )
                .await
                .context("failed to init room")?,
        );

        Ok(self.clone())
    }

    pub async fn connect_room(&mut self) -> Result<Self> {
        let room = self.room.as_ref().context("room is missing")?;
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs()
            .to_string();

        let response = self
            .grpc_service
            .join_shuffle_room(JoinShuffleRoomRequest {
                utxo: room.utxo.id.to_string(),
                sign: room
                    .ecdsa_private_key
                    .sign_message(room.utxo.id.to_string() + timestamp.as_str())
                    .await
                    .context("failed to sign with ecdsa priv key")?
                    .to_string(),
                timestamp: timestamp.as_str().parse()?,
            })
            .await
            .context("got not successful status from shuffle-service")?;

        self.jwt = Some(
            response
                .into_inner()
                .access_jwt
                .context("access jwt is absent in issuer request")?
                .jwt,
        );

        Ok(self.clone())
    }
}
