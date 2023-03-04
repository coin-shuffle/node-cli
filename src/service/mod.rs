use coin_shuffle_contracts_bindings::utxo::Connector;
use coin_shuffle_core::node::{
    room::Room, signer::Signer, storage::RoomMemoryStorage, Node as Core,
};
use coin_shuffle_protos::v1::{
    shuffle_event::Body, shuffle_service_client::ShuffleServiceClient, ConnectShuffleRoomRequest,
    EncodedOutputs, IsReadyForShuffleRequest, JoinShuffleRoomRequest,
    RsaPublicKey as ProtoRSAPublicKey, ShuffleError, ShuffleEvent, ShuffleInfo,
    ShuffleRoundRequest, ShuffleTxHash, SignShuffleTxRequest, TxSigningOutputs,
};
use ethers::{
    providers::{Http, Provider},
    signers::LocalWallet,
    types::{Address, U256},
};
use eyre::{Context, ContextCompat, Result};
use open_fastrlp::Encodable;
use rsa::{
    pkcs1::DecodeRsaPrivateKey,
    {BigUint, PublicKeyParts, RsaPrivateKey, RsaPublicKey},
};
use signer::DirectSigner;
use std::{
    fs::read_to_string,
    str::FromStr,
    time::{Duration, SystemTime},
};
use tonic::{codec::Streaming, transport::Channel};

mod signer;

const U256_BYTES: usize = 32;
const TIMESTAMP_BYTES: usize = 8;
const MESSAGE_LEN: usize = U256_BYTES + TIMESTAMP_BYTES;

#[derive(Debug, Clone)]
pub struct Node {
    inner: Core<DirectSigner, RoomMemoryStorage<DirectSigner>, Connector<Provider<Http>>>,
    grpc_service: ShuffleServiceClient<Channel>,
    room: Option<Room<DirectSigner>>,
    jwt: String,
}

impl Node {
    pub fn new(
        rpc_url: String,
        utxo_address: String,
        grpc_service: ShuffleServiceClient<Channel>,
    ) -> Result<Self> {
        Ok(Self {
            inner: Core::new(
                RoomMemoryStorage::new(),
                Connector::from_raw(rpc_url, utxo_address)
                    .context("failed to init connector from raw")?,
            ),
            grpc_service,
            room: None,
            jwt: String::default(),
        })
    }

    pub async fn init_shuffle_room(
        &mut self,
        utxo_id: U256,
        output_address_raw: String,
        rsa_priv_path: String,
        ecdsa_priv_path: String,
    ) -> Result<()> {
        log::info!("[NODE] initializing room...");

        let rsa_private_key = RsaPrivateKey::from_pkcs1_pem(
            read_to_string(rsa_priv_path)
                .context("failed to read rsa priv key from file")?
                .as_str(),
        )
        .context("failed to parse rsa private key")?;

        let ecdsa_private_key = LocalWallet::from_str(
            read_to_string(ecdsa_priv_path)
                .context("failed to read ecdsa priv key from file")?
                .as_str()
                .trim(),
        )
        .context("failed to parse ecdsa priv key")?;

        let output_address = Address::from_str(output_address_raw.as_str())
            .context("failed to parse output address")?;

        self.room = Some(
            self.inner
                .init_room(
                    utxo_id,
                    output_address.as_bytes().to_vec(),
                    rsa_private_key,
                    DirectSigner::new(ecdsa_private_key),
                )
                .await
                .context("failed to init room")?,
        );

        Ok(())
    }

    pub async fn join_shuffle_room(&mut self) -> Result<()> {
        log::info!("[NODE] connecting to room...");

        let room = self.room.as_ref().context("room is missing")?;
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs();

        let mut message = vec![0u8; MESSAGE_LEN];

        room.utxo.id.to_big_endian(&mut message[0..U256_BYTES]);
        message[U256_BYTES..MESSAGE_LEN].copy_from_slice(&timestamp.to_be_bytes());

        let mut signature = Vec::new();
        room.signer
            .sign_message(message)
            .await
            .context("failed to sign with ecdsa priv key")?
            .encode(&mut signature);

        let mut utxo_id = vec![0u8; U256_BYTES];

        room.utxo.id.to_big_endian(&mut utxo_id);

        let response = self
            .grpc_service
            .join_shuffle_room(with_auth(
                self.jwt.clone(),
                JoinShuffleRoomRequest {
                    utxo_id,
                    timestamp,
                    signature,
                },
            ))
            .await
            .context("got unsuccessful status from shuffle-service")?;

        self.jwt = response.into_inner().room_access_token;

        Ok(())
    }

    pub async fn wait_shuffle(&mut self) -> Result<()> {
        log::info!("[NODE] waiting shuffle start...");

        let mut is_ready = false;

        while !is_ready {
            tokio::time::sleep(Duration::from_secs(30)).await;
            let response = self
                .grpc_service
                .is_ready_for_shuffle(with_auth(self.jwt.clone(), IsReadyForShuffleRequest {}))
                .await
                .context("failed to check shuffle room status")?;

            is_ready = response.into_inner().ready;
        }

        Ok(())
    }

    pub async fn connect_room(&mut self) -> Result<Streaming<ShuffleEvent>> {
        log::info!("[NODE] room is found, start shuffling...");

        let rsa_public_key = self
            .room
            .clone()
            .context("room is absent")?
            .rsa_private_key
            .to_public_key();

        Ok(self
            .grpc_service
            .connect_shuffle_room(with_auth(
                self.jwt.clone(),
                ConnectShuffleRoomRequest {
                    public_key: Some(ProtoRSAPublicKey {
                        modulus: rsa_public_key.n().to_bytes_be(),
                        exponent: rsa_public_key.e().to_bytes_be(),
                    }),
                },
            ))
            .await
            .context("failed to connect shuffle room")?
            .into_inner())
    }

    pub(crate) async fn shuffling(
        &mut self,
        mut stream: Streaming<ShuffleEvent>,
    ) -> Result<String> {
        while let Ok(Some(event)) = stream.message().await {
            log::debug!("[NODE] got event: {:?}", event);
            let Some(data) = event.body else {
                continue;
            };

            match data {
                Body::ShuffleInfo(event_body) => {
                    log::info!("[NODE] shuffle room found...");
                    self.event_shuffle_info(event_body).await?
                }
                Body::EncodedOutputs(event_body) => {
                    log::debug!("[NODE] received encoded outputs");
                    self.event_encoded_outputs(event_body).await?
                }
                Body::TxSigningOutputs(event_body) => {
                    log::debug!("[NODE] received transaction signing outputs");
                    self.event_signing_outputs(event_body).await?
                }
                Body::ShuffleTxHash(event_body) => {
                    log::debug!("[NODE] received transaction hash");
                    self.event_tx_hash(event_body).await?
                }
                Body::Error(event_body) => {
                    log::debug!("[NODE] received error from shuffle-service");
                    self.event_error(event_body).await?
                }
            }
        }

        Ok("".to_string())
    }

    async fn event_shuffle_info(&mut self, event_body: ShuffleInfo) -> Result<()> {
        self.jwt = event_body.shuffle_access_token;

        let mut participants_public_keys = Vec::<RsaPublicKey>::new();
        for public_key_raw in event_body.public_keys_list {
            let public_key = RsaPublicKey::new(
                BigUint::from_bytes_be(public_key_raw.modulus.as_slice()),
                BigUint::from_bytes_be(public_key_raw.exponent.as_slice()),
            )
            .context("failed to parse rsa public key")?;
            participants_public_keys.insert(0, public_key);
        }

        self.inner
            .update_shuffle_info(
                participants_public_keys,
                self.room.clone().context("room is absent")?.utxo.id,
            )
            .await
            .context("failed to update room shuffle info")?;

        Ok(())
    }

    async fn event_encoded_outputs(&mut self, event_body: EncodedOutputs) -> Result<()> {
        let decoded_outputs = self
            .inner
            .shuffle_round(
                event_body.outputs,
                self.room.clone().context("room is absent")?.utxo.id,
            )
            .await
            .context("failed to do shuffle round")?;

        self.grpc_service
            .shuffle_round(with_auth(
                self.jwt.clone(),
                ShuffleRoundRequest {
                    encoded_outputs: decoded_outputs,
                },
            ))
            .await
            .context("failed to send shuffle round result")?;

        Ok(())
    }

    async fn event_signing_outputs(&mut self, event_body: TxSigningOutputs) -> Result<()> {
        let signature = self
            .inner
            .sign_tx(
                self.room.clone().context("room is absent")?.utxo.id,
                event_body.outputs,
            )
            .await
            .context("failed to sign outputs")?;

        self.grpc_service
            .sign_shuffle_tx(with_auth(
                self.jwt.clone(),
                SignShuffleTxRequest { signature },
            ))
            .await
            .context("failed to send signed shuffle transaction")?;

        Ok(())
    }

    async fn event_tx_hash(&mut self, event_body: ShuffleTxHash) -> Result<()> {
        log::info!(
            "[NODE] utxo successfully shuffled, tx hash: {0}",
            ethers::types::H160::from_slice(event_body.tx_hash.as_slice()).to_string()
        );

        Ok(())
    }

    async fn event_error(&self, event_body: ShuffleError) -> Result<()> {
        // TODO: Add error handling
        log::info!(
            "[NODE] failed to shuffle, got error from shuffle error: {0}",
            event_body.error
        );

        Ok(())
    }
}

fn with_auth<T>(token: String, request: T) -> tonic::Request<T> {
    let mut request = tonic::Request::new(request);
    request.metadata_mut().insert(
        "authorization",
        ("Bearer ".to_owned() + token.as_str()).parse().unwrap(),
    );

    request
}
