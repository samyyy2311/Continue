// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use capabilities::{evaluate_capability, CapabilityQuery};
use limits::{
    MAX_CATALOG_ITEM_ID_BYTES, MAX_CATALOG_PAGE_SIZE, MAX_FRAME_CATALOG_BYTES,
    MAX_FRAME_THUMBNAIL_BYTES,
};
use protocol::v1::{
    catalog_envelope::Payload, CatalogEnvelope, CatalogQuery, CatalogResponse, ThumbnailRequest,
    ThumbnailResponse,
};
use transport::{read_msg, write_msg};

use crate::error::SessionError;

#[derive(Default, Clone)]
pub struct CatalogDispatcher;

impl CatalogDispatcher {
    pub fn new() -> Self {
        Self
    }

    pub fn validate_query(query: &CatalogQuery) -> Result<(), SessionError> {
        if query.limit > MAX_CATALOG_PAGE_SIZE {
            return Err(SessionError::CatalogValidation(format!(
                "Catalog query limit {} exceeds maximum page size of {}",
                query.limit, MAX_CATALOG_PAGE_SIZE
            )));
        }
        Ok(())
    }

    pub fn validate_thumbnail_request(req: &ThumbnailRequest) -> Result<(), SessionError> {
        if req.item_id.is_empty() {
            return Err(SessionError::CatalogValidation(
                "Thumbnail item ID cannot be empty".into(),
            ));
        }

        if req.item_id.len() > MAX_CATALOG_ITEM_ID_BYTES {
            return Err(SessionError::CatalogValidation(format!(
                "Thumbnail item ID length {} exceeds maximum allowed of {} bytes",
                req.item_id.len(),
                MAX_CATALOG_ITEM_ID_BYTES
            )));
        }

        Ok(())
    }

    /// Sends a catalog query request and awaits the remote peer's catalog response.
    pub async fn query_catalog(
        &self,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        query: CatalogQuery,
        capability_query: &CapabilityQuery,
    ) -> Result<CatalogResponse, SessionError> {
        evaluate_capability(capability_query)?;
        Self::validate_query(&query)?;

        let envelope = CatalogEnvelope {
            payload: Some(Payload::Query(query)),
        };

        write_msg(send_stream, &envelope, MAX_FRAME_CATALOG_BYTES).await?;
        let resp_env: CatalogEnvelope = read_msg(recv_stream, MAX_FRAME_CATALOG_BYTES).await?;

        match resp_env.payload {
            Some(Payload::Response(resp)) => Ok(resp),
            _ => Err(SessionError::UnexpectedMessage),
        }
    }

    /// Sends a thumbnail request and awaits the remote peer's thumbnail response.
    pub async fn request_thumbnail(
        &self,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        req: ThumbnailRequest,
        capability_query: &CapabilityQuery,
    ) -> Result<ThumbnailResponse, SessionError> {
        evaluate_capability(capability_query)?;
        Self::validate_thumbnail_request(&req)?;

        let envelope = CatalogEnvelope {
            payload: Some(Payload::ThumbReq(req)),
        };

        write_msg(send_stream, &envelope, MAX_FRAME_CATALOG_BYTES).await?;
        let resp_env: CatalogEnvelope = read_msg(recv_stream, MAX_FRAME_THUMBNAIL_BYTES).await?;

        match resp_env.payload {
            Some(Payload::ThumbResp(resp)) => Ok(resp),
            _ => Err(SessionError::UnexpectedMessage),
        }
    }

    /// Receives a catalog envelope, delegates to query or thumbnail handler, and replies.
    pub async fn receive_envelope<Q, T>(
        &self,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        capability_query: &CapabilityQuery,
        mut on_query: Q,
        mut on_thumb: T,
    ) -> Result<CatalogEnvelope, SessionError>
    where
        Q: FnMut(CatalogQuery) -> CatalogResponse,
        T: FnMut(ThumbnailRequest) -> ThumbnailResponse,
    {
        evaluate_capability(capability_query)?;

        let envelope: CatalogEnvelope = read_msg(recv_stream, MAX_FRAME_CATALOG_BYTES).await?;

        match envelope.payload {
            Some(Payload::Query(ref query)) => {
                Self::validate_query(query)?;
                let resp = on_query(*query);
                let resp_env = CatalogEnvelope {
                    payload: Some(Payload::Response(resp)),
                };
                write_msg(send_stream, &resp_env, MAX_FRAME_CATALOG_BYTES).await?;
                Ok(envelope)
            }
            Some(Payload::ThumbReq(ref req)) => {
                Self::validate_thumbnail_request(req)?;
                let resp = on_thumb(req.clone());
                let resp_env = CatalogEnvelope {
                    payload: Some(Payload::ThumbResp(resp)),
                };
                write_msg(send_stream, &resp_env, MAX_FRAME_THUMBNAIL_BYTES).await?;
                Ok(envelope)
            }
            Some(Payload::Response(_)) | Some(Payload::ThumbResp(_)) | None => {
                Err(SessionError::UnexpectedMessage)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::v1::{CatalogCategory, CatalogItem};
    use protocol::CapabilityId;
    use transport::{create_client_endpoint, create_server_endpoint, TransportCertificate};

    #[tokio::test]
    async fn catalog_query_roundtrip() {
        let server_cert = TransportCertificate::generate().unwrap();
        let client_cert = TransportCertificate::generate().unwrap();

        let server_tls = server_cert
            .build_pinned_server_tls(client_cert.spki_hash)
            .unwrap();
        let client_tls = client_cert
            .build_pinned_client_tls(server_cert.spki_hash)
            .unwrap();

        let server_ep = create_server_endpoint("127.0.0.1:0".parse().unwrap(), server_tls).unwrap();
        let server_addr = server_ep.local_addr().unwrap();
        let client_ep = create_client_endpoint("0.0.0.0:0".parse().unwrap(), client_tls).unwrap();

        let server_task = tokio::spawn(async move {
            server_ep.wait_idle().await;
            let conn = server_ep.accept().await.unwrap().await.unwrap();
            let (mut send, mut recv) = conn.accept_bi().await.unwrap();

            let query = CapabilityQuery::negotiated(CapabilityId::FILE_CATALOG, true);
            let dispatcher = CatalogDispatcher::new();
            let result = dispatcher
                .receive_envelope(
                    &mut send,
                    &mut recv,
                    &query,
                    |q| {
                        assert_eq!(q.category, CatalogCategory::Photos as i32);
                        CatalogResponse {
                            items: vec![CatalogItem {
                                item_id: "photo_001".into(),
                                file_name: "sunset.jpg".into(),
                                size_bytes: 2048576,
                                timestamp: 1700000000,
                                mime_type: "image/jpeg".into(),
                            }],
                            total_count: 1,
                        }
                    },
                    |_| panic!("Unexpected thumbnail request"),
                )
                .await;
            (result, conn)
        });

        let conn = client_ep
            .connect(server_addr, "localhost")
            .unwrap()
            .await
            .unwrap();
        let (mut send, mut recv) = conn.open_bi().await.unwrap();

        let query = CapabilityQuery::negotiated(CapabilityId::FILE_CATALOG, true);
        let dispatcher = CatalogDispatcher::new();
        let response = dispatcher
            .query_catalog(
                &mut send,
                &mut recv,
                CatalogQuery {
                    category: CatalogCategory::Photos as i32,
                    limit: 50,
                    offset: 0,
                },
                &query,
            )
            .await
            .unwrap();

        assert_eq!(response.items.len(), 1);
        assert_eq!(response.items[0].file_name, "sunset.jpg");
        assert_eq!(response.items[0].size_bytes, 2048576);

        let (server_res, _conn) = server_task.await.unwrap();
        assert!(server_res.is_ok());
    }

    #[tokio::test]
    async fn thumbnail_request_roundtrip() {
        let server_cert = TransportCertificate::generate().unwrap();
        let client_cert = TransportCertificate::generate().unwrap();

        let server_tls = server_cert
            .build_pinned_server_tls(client_cert.spki_hash)
            .unwrap();
        let client_tls = client_cert
            .build_pinned_client_tls(server_cert.spki_hash)
            .unwrap();

        let server_ep = create_server_endpoint("127.0.0.1:0".parse().unwrap(), server_tls).unwrap();
        let server_addr = server_ep.local_addr().unwrap();
        let client_ep = create_client_endpoint("0.0.0.0:0".parse().unwrap(), client_tls).unwrap();

        let server_task = tokio::spawn(async move {
            server_ep.wait_idle().await;
            let conn = server_ep.accept().await.unwrap().await.unwrap();
            let (mut send, mut recv) = conn.accept_bi().await.unwrap();

            let query = CapabilityQuery::negotiated(CapabilityId::FILE_CATALOG, true);
            let dispatcher = CatalogDispatcher::new();
            let result = dispatcher
                .receive_envelope(
                    &mut send,
                    &mut recv,
                    &query,
                    |_| panic!("Unexpected catalog query"),
                    |req| {
                        assert_eq!(req.item_id, "photo_001");
                        ThumbnailResponse {
                            item_id: req.item_id,
                            image_data: vec![0xFF, 0xD8, 0xFF, 0xE0],
                            mime_type: "image/jpeg".into(),
                            success: true,
                            error_message: String::new(),
                        }
                    },
                )
                .await;
            (result, conn)
        });

        let conn = client_ep
            .connect(server_addr, "localhost")
            .unwrap()
            .await
            .unwrap();
        let (mut send, mut recv) = conn.open_bi().await.unwrap();

        let query = CapabilityQuery::negotiated(CapabilityId::FILE_CATALOG, true);
        let dispatcher = CatalogDispatcher::new();
        let response = dispatcher
            .request_thumbnail(
                &mut send,
                &mut recv,
                ThumbnailRequest {
                    item_id: "photo_001".into(),
                    max_dimension: 256,
                },
                &query,
            )
            .await
            .unwrap();

        assert!(response.success);
        assert_eq!(response.item_id, "photo_001");
        assert_eq!(response.image_data, vec![0xFF, 0xD8, 0xFF, 0xE0]);

        let (server_res, _conn) = server_task.await.unwrap();
        assert!(server_res.is_ok());
    }

    #[tokio::test]
    async fn catalog_validation_rejects_oversized_limit() {
        let query = CatalogQuery {
            category: CatalogCategory::Unspecified as i32,
            limit: MAX_CATALOG_PAGE_SIZE + 10,
            offset: 0,
        };
        let err = CatalogDispatcher::validate_query(&query).unwrap_err();
        assert!(matches!(err, SessionError::CatalogValidation(_)));
    }
}
