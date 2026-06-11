//! Capability tokens negotiated between client and agent during the TTMP
//! handshake.
//!
//! Capabilities make the protocol forward-compatible: a newer client can ask
//! for features (hardware capture, audio, multi-monitor, session resume) that
//! an older agent does not understand, and the agent simply omits unknown
//! tokens from the negotiated set. Both sides then operate on the intersection,
//! falling back to the legacy PNG-over-TCP behaviour when nothing richer is
//! mutually supported.

/// Legacy single-frame PNG video (always supported, the historical default).
pub const CAP_VIDEO_PNG: &str = "video.png";
/// H.264 elementary stream (hardware or software encoded).
pub const CAP_VIDEO_H264: &str = "video.h264";
/// Opus-encoded remote audio playback.
pub const CAP_AUDIO_OPUS: &str = "audio.opus";
/// Keyboard / mouse input injection.
pub const CAP_INPUT: &str = "input";
/// Plain-text clipboard synchronisation.
pub const CAP_CLIPBOARD_TEXT: &str = "clipboard.text";
/// Image clipboard synchronisation.
pub const CAP_CLIPBOARD_IMAGE: &str = "clipboard.image";
/// File transfer channel.
pub const CAP_FILE_TRANSFER: &str = "file.transfer";
/// Multiple physical monitors can be enumerated and switched.
pub const CAP_MULTI_MONITOR: &str = "display.multi";
/// Hardware-accelerated screen capture is available (DXGI / PipeWire / SHM).
pub const CAP_HW_CAPTURE: &str = "capture.hardware";
/// The session can survive a transient network drop and be resumed with a
/// resume token instead of being torn down.
pub const CAP_SESSION_RESUME: &str = "session.resume";
/// Unattended access is permitted (no interactive consent prompt required).
pub const CAP_UNATTENDED: &str = "access.unattended";

/// The full set of capabilities this build of the agent can offer. The actual
/// per-connection set is the intersection with what the client requests and
/// what the host can physically do (e.g. a host with no audio device drops
/// [`CAP_AUDIO_OPUS`]).
pub fn server_capabilities() -> Vec<String> {
    [
        CAP_VIDEO_PNG,
        CAP_VIDEO_H264,
        CAP_INPUT,
        CAP_CLIPBOARD_TEXT,
        CAP_CLIPBOARD_IMAGE,
        CAP_FILE_TRANSFER,
        CAP_MULTI_MONITOR,
        CAP_HW_CAPTURE,
        CAP_SESSION_RESUME,
        CAP_UNATTENDED,
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// Compute the negotiated capability set: every token the client requested that
/// the server also supports, preserving the server's preference order. Unknown
/// tokens on either side are silently ignored, which is what keeps old and new
/// peers interoperable.
pub fn negotiate(client: &[String], server: &[String]) -> Vec<String> {
    server
        .iter()
        .filter(|cap| client.iter().any(|c| c == *cap))
        .cloned()
        .collect()
}

/// Convenience: does the negotiated set contain `cap`?
pub fn has(caps: &[String], cap: &str) -> bool {
    caps.iter().any(|c| c == cap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negotiate_returns_intersection_in_server_order() {
        let server = vec![
            CAP_VIDEO_H264.to_string(),
            CAP_VIDEO_PNG.to_string(),
            CAP_AUDIO_OPUS.to_string(),
        ];
        let client = vec![
            CAP_VIDEO_PNG.to_string(),
            CAP_VIDEO_H264.to_string(),
            "some.future.cap".to_string(),
        ];
        let negotiated = negotiate(&client, &server);
        // server order preserved, unknown client token dropped, audio dropped
        assert_eq!(
            negotiated,
            vec![CAP_VIDEO_H264.to_string(), CAP_VIDEO_PNG.to_string()]
        );
    }

    #[test]
    fn old_client_only_gets_png() {
        let server = server_capabilities();
        let legacy_client = vec![CAP_VIDEO_PNG.to_string(), CAP_INPUT.to_string()];
        let negotiated = negotiate(&legacy_client, &server);
        assert!(has(&negotiated, CAP_VIDEO_PNG));
        assert!(has(&negotiated, CAP_INPUT));
        assert!(!has(&negotiated, CAP_VIDEO_H264));
        assert!(!has(&negotiated, CAP_HW_CAPTURE));
    }

    #[test]
    fn has_is_consistent() {
        let caps = vec![CAP_MULTI_MONITOR.to_string()];
        assert!(has(&caps, CAP_MULTI_MONITOR));
        assert!(!has(&caps, CAP_AUDIO_OPUS));
    }
}
