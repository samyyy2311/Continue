// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use capabilities::{evaluate_capability, CapabilityQuery};
use limits::{
    MAX_FRAME_DESKTOP_STREAM_BYTES, MAX_STREAM_PACKAGE_NAME_BYTES, MAX_STREAM_SESSION_ID_BYTES,
};
use protocol::v1::{
    desktop_stream_envelope::Payload, DesktopInputEvent, DesktopStreamAck, DesktopStreamControl,
    DesktopStreamEnvelope, DesktopStreamFrame, DesktopStreamStartRequest,
    DesktopStreamStartResponse, DesktopStreamStatus,
};
use transport::{read_msg, write_msg};

use crate::error::SessionError;

#[derive(Default, Clone)]
pub struct DesktopStreamDispatcher;

impl DesktopStreamDispatcher {
    pub fn new() -> Self {
        Self
    }

    pub fn validate_session_id(session_id: &str) -> Result<(), SessionError> {
        if session_id.is_empty() {
            return Err(SessionError::DesktopStreamValidation(
                "Session ID cannot be empty".into(),
            ));
        }
        if session_id.len() > MAX_STREAM_SESSION_ID_BYTES {
            return Err(SessionError::DesktopStreamValidation(format!(
                "Session ID length {} exceeds limit {}",
                session_id.len(),
                MAX_STREAM_SESSION_ID_BYTES
            )));
        }
        Ok(())
    }

    pub fn validate_start_request(req: &DesktopStreamStartRequest) -> Result<(), SessionError> {
        Self::validate_session_id(&req.session_id)?;

        if req.target_package_name.len() > MAX_STREAM_PACKAGE_NAME_BYTES {
            return Err(SessionError::DesktopStreamValidation(format!(
                "Package name length {} exceeds limit {}",
                req.target_package_name.len(),
                MAX_STREAM_PACKAGE_NAME_BYTES
            )));
        }

        if req.requested_width == 0 || req.requested_height == 0 {
            return Err(SessionError::DesktopStreamValidation(
                "Requested dimensions must be greater than zero".into(),
            ));
        }

        if req.requested_width > 7680 || req.requested_height > 4320 {
            return Err(SessionError::DesktopStreamValidation(
                "Requested dimensions exceed 8K boundary".into(),
            ));
        }

        Ok(())
    }

    pub fn validate_input_event(event: &DesktopInputEvent) -> Result<(), SessionError> {
        Self::validate_session_id(&event.session_id)?;
        Ok(())
    }

    pub fn validate_control(control: &DesktopStreamControl) -> Result<(), SessionError> {
        Self::validate_session_id(&control.session_id)?;
        Ok(())
    }

    /// Sends a desktop streaming start request to peer and waits for its response.
    pub async fn start_stream(
        &self,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        request: DesktopStreamStartRequest,
        query: &CapabilityQuery,
    ) -> Result<DesktopStreamStartResponse, SessionError> {
        evaluate_capability(query)?;
        Self::validate_start_request(&request)?;

        let envelope = DesktopStreamEnvelope {
            payload: Some(Payload::StartRequest(request)),
        };

        write_msg(send_stream, &envelope, MAX_FRAME_DESKTOP_STREAM_BYTES).await?;
        let resp_envelope: DesktopStreamEnvelope =
            read_msg(recv_stream, MAX_FRAME_DESKTOP_STREAM_BYTES).await?;

        match resp_envelope.payload {
            Some(Payload::StartResponse(resp)) => Ok(resp),
            _ => Err(SessionError::UnexpectedMessage),
        }
    }

    /// Sends an input event (pointer or key) to the active desktop stream.
    pub async fn send_input_event(
        &self,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        event: DesktopInputEvent,
        query: &CapabilityQuery,
    ) -> Result<DesktopStreamAck, SessionError> {
        evaluate_capability(query)?;
        Self::validate_input_event(&event)?;

        let envelope = DesktopStreamEnvelope {
            payload: Some(Payload::InputEvent(event)),
        };

        write_msg(send_stream, &envelope, MAX_FRAME_DESKTOP_STREAM_BYTES).await?;
        let resp: DesktopStreamEnvelope =
            read_msg(recv_stream, MAX_FRAME_DESKTOP_STREAM_BYTES).await?;

        match resp.payload {
            Some(Payload::Ack(ack)) => Ok(ack),
            _ => Err(SessionError::UnexpectedMessage),
        }
    }

    /// Sends stream control action (stop, request keyframe, or resize).
    pub async fn send_control(
        &self,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        control: DesktopStreamControl,
        query: &CapabilityQuery,
    ) -> Result<DesktopStreamAck, SessionError> {
        evaluate_capability(query)?;
        Self::validate_control(&control)?;

        let envelope = DesktopStreamEnvelope {
            payload: Some(Payload::Control(control)),
        };

        write_msg(send_stream, &envelope, MAX_FRAME_DESKTOP_STREAM_BYTES).await?;
        let resp: DesktopStreamEnvelope =
            read_msg(recv_stream, MAX_FRAME_DESKTOP_STREAM_BYTES).await?;

        match resp.payload {
            Some(Payload::Ack(ack)) => Ok(ack),
            _ => Err(SessionError::UnexpectedMessage),
        }
    }

    /// Receives and processes a desktop stream envelope on an incoming capability stream.
    #[allow(clippy::too_many_arguments)]
    pub async fn receive_envelope<FStart, FInput, FControl, FFrame>(
        &self,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        query: &CapabilityQuery,
        on_start: FStart,
        mut on_input: FInput,
        mut on_control: FControl,
        mut on_frame: FFrame,
    ) -> Result<(), SessionError>
    where
        FStart: FnOnce(DesktopStreamStartRequest) -> DesktopStreamStartResponse,
        FInput: FnMut(DesktopInputEvent) -> DesktopStreamAck,
        FControl: FnMut(DesktopStreamControl) -> DesktopStreamAck,
        FFrame: FnMut(DesktopStreamFrame),
    {
        evaluate_capability(query)?;

        let envelope: DesktopStreamEnvelope =
            read_msg(recv_stream, MAX_FRAME_DESKTOP_STREAM_BYTES).await?;

        match envelope.payload {
            Some(Payload::StartRequest(req)) => {
                let resp = if let Err(e) = Self::validate_start_request(&req) {
                    DesktopStreamStartResponse {
                        session_id: req.session_id,
                        status: DesktopStreamStatus::Error as i32,
                        error_message: e.to_string(),
                        actual_width: 0,
                        actual_height: 0,
                        actual_dpi: 0,
                        selected_codec: 0,
                        display_id: 0,
                    }
                } else {
                    on_start(req)
                };

                let resp_env = DesktopStreamEnvelope {
                    payload: Some(Payload::StartResponse(resp)),
                };
                write_msg(send_stream, &resp_env, MAX_FRAME_DESKTOP_STREAM_BYTES).await?;
            }
            Some(Payload::InputEvent(input)) => {
                let ack = if let Err(e) = Self::validate_input_event(&input) {
                    DesktopStreamAck {
                        session_id: input.session_id,
                        success: false,
                        error_message: e.to_string(),
                    }
                } else {
                    on_input(input)
                };

                let resp_env = DesktopStreamEnvelope {
                    payload: Some(Payload::Ack(ack)),
                };
                write_msg(send_stream, &resp_env, MAX_FRAME_DESKTOP_STREAM_BYTES).await?;
            }
            Some(Payload::Control(ctl)) => {
                let ack = if let Err(e) = Self::validate_control(&ctl) {
                    DesktopStreamAck {
                        session_id: ctl.session_id,
                        success: false,
                        error_message: e.to_string(),
                    }
                } else {
                    on_control(ctl)
                };

                let resp_env = DesktopStreamEnvelope {
                    payload: Some(Payload::Ack(ack)),
                };
                write_msg(send_stream, &resp_env, MAX_FRAME_DESKTOP_STREAM_BYTES).await?;
            }
            Some(Payload::Frame(frame)) => {
                on_frame(frame);
            }
            _ => return Err(SessionError::UnexpectedMessage),
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::CapabilityId;

    #[test]
    fn validates_start_request_parameters() {
        let valid = DesktopStreamStartRequest {
            session_id: "stream-123".into(),
            target_package_name: "".into(),
            requested_width: 1920,
            requested_height: 1080,
            requested_dpi: 240,
            max_fps: 60,
            preferred_codec: 1,
        };
        assert!(DesktopStreamDispatcher::validate_start_request(&valid).is_ok());

        let empty_id = DesktopStreamStartRequest {
            session_id: "".into(),
            ..valid.clone()
        };
        assert!(DesktopStreamDispatcher::validate_start_request(&empty_id).is_err());

        let zero_dim = DesktopStreamStartRequest {
            requested_width: 0,
            ..valid.clone()
        };
        assert!(DesktopStreamDispatcher::validate_start_request(&zero_dim).is_err());

        let oversized_dim = DesktopStreamStartRequest {
            requested_width: 8000,
            ..valid.clone()
        };
        assert!(DesktopStreamDispatcher::validate_start_request(&oversized_dim).is_err());

        let long_package = DesktopStreamStartRequest {
            target_package_name: "a".repeat(300),
            ..valid
        };
        assert!(DesktopStreamDispatcher::validate_start_request(&long_package).is_err());
    }

    #[test]
    fn validates_input_events() {
        let valid = DesktopInputEvent {
            session_id: "stream-123".into(),
            event_type: 1,
            x: 100,
            y: 200,
            button: 0,
            key_code: 0,
            scroll_dx: 0,
            scroll_dy: 0,
            key_text: "".into(),
        };
        assert!(DesktopStreamDispatcher::validate_input_event(&valid).is_ok());

        let empty = DesktopInputEvent {
            session_id: "".into(),
            ..valid
        };
        assert!(DesktopStreamDispatcher::validate_input_event(&empty).is_err());
    }

    #[test]
    fn capability_denied_aborts() {
        let query = CapabilityQuery::negotiated(CapabilityId::DESKTOP_STREAM, false);
        let req = DesktopStreamStartRequest {
            session_id: "s-1".into(),
            target_package_name: "".into(),
            requested_width: 1280,
            requested_height: 720,
            requested_dpi: 160,
            max_fps: 30,
            preferred_codec: 1,
        };

        // Validate directly fails on capability gate
        assert!(evaluate_capability(&query).is_err());
        assert!(DesktopStreamDispatcher::validate_session_id(&req.session_id).is_ok());
    }
}
