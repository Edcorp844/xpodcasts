pub fn handel_gst_resource_error(error: gst::ResourceError) -> String {
    let error_msg = match error {
        gst::ResourceError::NotFound => "The audio stream URL could not be found or resolved.",
        gst::ResourceError::Read => "Network read failure occurred during playback.",
        gst::ResourceError::NotAuthorized => {
            "Access denied: You don't have permission to play this stream."
        }
        gst::ResourceError::Busy => {
            "The podcast server or media resource is currently too busy to stream."
        }
        gst::ResourceError::Failed => "A critical system error prevented the podcast from loading.",
        gst::ResourceError::TooLazy => {
            "The streaming subsystem initialization stalled or timed out."
        }
        gst::ResourceError::OpenRead => "Failed to open the podcast audio stream link for reading.",
        gst::ResourceError::OpenWrite => {
            "Failed to establish a writable storage buffer for downloading."
        }
        gst::ResourceError::OpenReadWrite => {
            "Unable to access the podcast data stream with read/write permissions."
        }
        gst::ResourceError::Close => {
            "An error occurred while terminating the podcast stream connection."
        }
        gst::ResourceError::Write => {
            "Could not write podcast cache data to your local disk storage."
        }
        gst::ResourceError::Seek => {
            "Unable to skip ahead or rewind; scrubbing is unsupported for this stream."
        }
        gst::ResourceError::Sync => {
            "Playback synchronization failure: The network stream lost pacing."
        }
        gst::ResourceError::Settings => "Invalid system audio or player configuration parameters.",
        gst::ResourceError::NoSpaceLeft => {
            "Your device storage is completely full. Cannot cache or download podcast."
        }

        gst::ResourceError::__Unknown(code) => {
            return format!(
                "An undocumented low-level system error occurred (Code: {}).",
                code
            );
        }
        _ => "Unknown Error in Playing Audio resource",
    };

    error_msg.to_string()
}

pub fn handel_gst_stream_error(error: gst::StreamError) -> String {
    let error_msg = match error {
        gst::StreamError::Failed => {
            "A critical error occurred while processing the podcast audio stream."
        }
        gst::StreamError::TooLazy => {
            "The audio streaming engine took too long to respond and timed out."
        }
        gst::StreamError::NotImplemented => {
            "The stream uses an unhandled player capability or feature."
        }
        gst::StreamError::TypeNotFound => {
            "Could not determine the audio format of this podcast link."
        }
        gst::StreamError::WrongType => {
            "The file type at this link does not appear to be a valid audio stream."
        }
        gst::StreamError::CodecNotFound => {
            "Missing audio decoder: Your system lacks the codec required to play this format."
        }
        gst::StreamError::Decode => {
            "Failed to decode the audio track. The file data may be corrupted."
        }
        gst::StreamError::Encode => {
            "An internal engine error occurred while encoding audio buffers."
        }
        gst::StreamError::Demux => {
            "Failed to split the incoming media stream into playable audio channels."
        }
        gst::StreamError::Mux => "An internal container format matching error occurred.",
        gst::StreamError::Format => {
            "The podcast stream contains data with an incompatible or unrecognized structural format."
        }
        gst::StreamError::Decrypt => "This stream is encrypted, and decryption processing failed.",
        gst::StreamError::DecryptNokey => {
            "This podcast content is protected, and a decryption key was not found."
        }
        // Safely processes unknown codes passed down from the underlying C framework
        gst::StreamError::__Unknown(code) => {
            return format!(
                "An undocumented low-level stream error occurred (Code: {}).",
                code
            );
        }
        _ => "Unknown treaming error ",
    };

    error_msg.to_string()
}

pub fn handel_gst_core_error(error: gst::CoreError) -> String {
    match error {
        gst::CoreError::Failed => {
            "The streaming connection dropped unexpectedly. Please try playing it again.".to_string()
        }
        // This is exactly what "Internal data stream error / reason error -5" maps to
        gst::CoreError::Event | gst::CoreError::Pad => {
            "The podcast server sent unreadable audio data, or the download stalled.".to_string()
        }
        _ => "An internal player pipeline error occurred.".to_string(),
    }
}