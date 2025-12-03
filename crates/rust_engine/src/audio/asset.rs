//! Audio asset types for loading and managing audio data
//!
//! Provides AudioAsset for storing decoded audio samples that can be
//! loaded through the AssetManager system.

use crate::assets::{Asset, AssetError};

/// Audio asset containing decoded audio samples
/// 
/// This stores the raw audio file bytes which can be decoded on-demand
/// by the audio backend during playback. Supports WAV, OGG, MP3, and FLAC formats.
#[derive(Clone)]
pub struct AudioAsset {
    /// Raw audio file data (encoded format)
    data: Vec<u8>,
    /// Original file format for debugging
    #[allow(dead_code)]
    format: AudioFormat,
}

/// Supported audio formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFormat {
    /// WAV uncompressed
    Wav,
    /// OGG Vorbis compressed
    Ogg,
    /// MP3 compressed
    Mp3,
    /// FLAC lossless
    Flac,
    /// Unknown format
    Unknown,
}

impl AudioAsset {
    /// Create a new audio asset from raw bytes
    ///
    /// # Arguments
    /// * `data` - Raw audio file bytes
    /// * `format` - Audio format
    pub fn new(data: Vec<u8>, format: AudioFormat) -> Self {
        Self { data, format }
    }

    /// Get the raw audio data
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Detect audio format from file extension or magic bytes
    fn detect_format(bytes: &[u8]) -> AudioFormat {
        if bytes.len() < 4 {
            return AudioFormat::Unknown;
        }

        // Check magic bytes
        match &bytes[0..4] {
            b"RIFF" => AudioFormat::Wav,
            b"OggS" => AudioFormat::Ogg,
            b"fLaC" => AudioFormat::Flac,
            // MP3 can start with ID3 tag or frame sync
            [0xFF, 0xFB, _, _] | [0xFF, 0xFA, _, _] => AudioFormat::Mp3,
            [b'I', b'D', b'3', _] => AudioFormat::Mp3,
            _ => AudioFormat::Unknown,
        }
    }
}

impl Asset for AudioAsset {
    fn from_bytes(bytes: &[u8]) -> Result<Self, AssetError> 
    where 
        Self: Sized 
    {
        if bytes.is_empty() {
            return Err(AssetError::InvalidData("Empty audio file".to_string()));
        }

        // Detect format
        let format = Self::detect_format(bytes);
        
        if format == AudioFormat::Unknown {
            return Err(AssetError::InvalidData("Unknown audio format".to_string()));
        }

        // Store the original encoded data for later playback
        // Validation will happen during playback when Rodio attempts to decode
        Ok(AudioAsset::new(bytes.to_vec(), format))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_detection() {
        // WAV format
        let wav_bytes = b"RIFF....WAVE";
        assert_eq!(AudioAsset::detect_format(wav_bytes), AudioFormat::Wav);

        // OGG format
        let ogg_bytes = b"OggS....";
        assert_eq!(AudioAsset::detect_format(ogg_bytes), AudioFormat::Ogg);

        // FLAC format
        let flac_bytes = b"fLaC....";
        assert_eq!(AudioAsset::detect_format(flac_bytes), AudioFormat::Flac);

        // Unknown format
        let unknown = b"ABCD";
        assert_eq!(AudioAsset::detect_format(unknown), AudioFormat::Unknown);
    }

    #[test]
    fn test_empty_data_fails() {
        let result = AudioAsset::from_bytes(&[]);
        assert!(result.is_err());
    }
}
