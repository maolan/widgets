use std::io;
use std::path::Path;

use symphonia::core::codecs::CodecParameters as SymphoniaCodecParameters;
use symphonia::core::codecs::audio::{AudioCodecParameters, AudioDecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

/// Decode a WAV file to interleaved `f32` samples.
///
/// Returns `(samples, channels, sample_rate)`. Samples are normalised to the
/// `[-1.0, 1.0]` range and are stored interleaved (`ch0, ch1, ...`).
pub fn read_wav_f32(path: &Path) -> io::Result<(Vec<f32>, usize, u32)> {
    let file = std::fs::File::open(path)
        .map_err(|e| io::Error::other(format!("Failed to open '{}': {e}", path.display())))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let format_opts = FormatOptions::default();
    let metadata_opts = MetadataOptions::default();
    let decoder_opts = AudioDecoderOptions::default();

    let mut format: Box<dyn FormatReader> = symphonia::default::get_probe()
        .probe(&hint, mss, format_opts, metadata_opts)
        .map_err(|e| {
            io::Error::other(format!(
                "Symphonia failed to probe format for '{}': {e}",
                path.display()
            ))
        })?;

    let track = format
        .default_track(TrackType::Audio)
        .or_else(|| format.tracks().first())
        .ok_or_else(|| {
            io::Error::other(format!("No usable audio track in '{}'", path.display()))
        })?;

    let codec_params: &AudioCodecParameters = track
        .codec_params
        .as_ref()
        .and_then(SymphoniaCodecParameters::audio)
        .ok_or_else(|| {
            io::Error::other(format!("No usable audio track in '{}'", path.display()))
        })?;

    let channels = codec_params
        .channels
        .as_ref()
        .map(|c| c.count())
        .unwrap_or(1);
    let sample_rate = codec_params.sample_rate.unwrap_or(48_000);
    let track_id = track.id;

    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(codec_params, &decoder_opts)
        .map_err(|e| {
            io::Error::other(format!(
                "Symphonia failed to create decoder for '{}': {e}",
                path.display()
            ))
        })?;

    let mut samples = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => break,
            Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                break;
            }
            Err(e) => {
                return Err(io::Error::other(format!(
                    "Symphonia read error for '{}': {e}",
                    path.display()
                )));
            }
        };

        if packet.track_id != track_id {
            continue;
        }

        let decoded = decoder.decode(&packet).map_err(|e| {
            io::Error::other(format!(
                "Symphonia decode error for '{}': {e}",
                path.display()
            ))
        })?;

        let mut packet_samples = Vec::new();
        decoded.copy_to_vec_interleaved(&mut packet_samples);
        samples.extend_from_slice(&packet_samples);
    }

    if samples.is_empty() {
        return Err(io::Error::other(format!(
            "No samples decoded from '{}'",
            path.display()
        )));
    }

    Ok((samples, channels, sample_rate))
}
