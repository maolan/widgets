use std::io;
use std::path::Path;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{CODEC_TYPE_NULL, DecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

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
    let decoder_opts = DecoderOptions::default();

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)
        .map_err(|e| {
            io::Error::other(format!(
                "Symphonia failed to probe format for '{}': {e}",
                path.display()
            ))
        })?;
    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .or_else(|| format.tracks().first())
        .ok_or_else(|| {
            io::Error::other(format!("No usable audio track in '{}'", path.display()))
        })?;

    let channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(1);
    let sample_rate = track.codec_params.sample_rate.unwrap_or(48_000);
    let track_id = track.id;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &decoder_opts)
        .map_err(|e| {
            io::Error::other(format!(
                "Symphonia failed to create decoder for '{}': {e}",
                path.display()
            ))
        })?;

    let mut sample_buf = None;
    let mut samples = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
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

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = decoder.decode(&packet).map_err(|e| {
            io::Error::other(format!(
                "Symphonia decode error for '{}': {e}",
                path.display()
            ))
        })?;

        if sample_buf.is_none() {
            let spec = *decoded.spec();
            sample_buf = Some(SampleBuffer::<f32>::new(decoded.capacity() as u64, spec));
        }
        let buf = sample_buf.as_mut().unwrap();
        buf.copy_interleaved_ref(decoded);
        samples.extend_from_slice(buf.samples());
    }

    if samples.is_empty() {
        return Err(io::Error::other(format!(
            "No samples decoded from '{}'",
            path.display()
        )));
    }

    Ok((samples, channels, sample_rate))
}
