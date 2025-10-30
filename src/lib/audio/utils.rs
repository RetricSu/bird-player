/// Audio processing utilities shared by state machine
use symphonia::core::codecs::FinalizeResult;
use symphonia::core::errors::{Error, Result};

pub fn ignore_end_of_stream_error(result: Result<()>) -> Result<()> {
    match result {
        Err(Error::IoError(err))
            if err.kind() == std::io::ErrorKind::UnexpectedEof
                && err.to_string() == "end of stream" =>
        {
            // Do not treat "end of stream" as a fatal error. It's the currently only way a
            // format reader can indicate the media is complete.
            Ok(())
        }
        _ => result,
    }
}

pub fn do_verification(finalization: FinalizeResult) -> Result<i32> {
    match finalization.verify_ok {
        Some(is_ok) => {
            // Got a verification result.
            tracing::info!("verification: {}", if is_ok { "passed" } else { "failed" });

            Ok(i32::from(!is_ok))
        }
        // Verification not enabled by user, or unsupported by the codec.
        _ => Ok(0),
    }
}
