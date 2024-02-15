use std::io;

use byteorder::{LittleEndian, ReadBytesExt};


#[derive(Debug)]
pub struct FormatChunk {
    pub audio_format: u16,
    pub num_channels: u16,
    pub sample_rate: u32,
    pub byte_rate: u32,
    pub block_align: u16,
    pub bits_per_sample: u16,
}

impl FormatChunk {
    fn from_buffer(mut buffer: &[u8]) -> io::Result<Self> {
        // Read the fields from the adjusted buffer
        let audio_format = buffer.read_u16::<LittleEndian>()?;
        let num_channels = buffer.read_u16::<LittleEndian>()?;
        let sample_rate = buffer.read_u32::<LittleEndian>()?;
        let byte_rate = buffer.read_u32::<LittleEndian>()?;
        let block_align = buffer.read_u16::<LittleEndian>()?;
        let bits_per_sample = buffer.read_u16::<LittleEndian>()?;

        Ok(FormatChunk {
            audio_format,
            num_channels,
            sample_rate,
            byte_rate,
            block_align,
            bits_per_sample,
        })
    }
}

#[derive(Debug)]
pub struct WaveData {
    pub format: FormatChunk,
    pub num_samples: usize,
    pub num_channels: u16,
    pub duration: f64,
}

impl WaveData {
    /**
     * Create a new `WaveData` instance from a buffer of bytes
     * @param buffer The buffer of bytes to read from
     * @returns A new `WaveData` instance
     * # Errors
     * Returns an error if the buffer is not a valid WAV file
     * or if the format chunk is not found
     * or if the data chunk is not found
     * or if the data chunk size is invalid
     * or if the chunk size is invalid
     * or if the chunk ID is invalid
     */
    pub fn from_buffer(buffer: &[u8]) -> io::Result<Self> {
        let mut format_chunk_offset = None;
        // let mut data_chunk_offset = None;
        // let mut format_chunk_size = 0;
        let mut data_chunk_size = 0;
        let mut offset = 12; // Start after the "RIFF" and "WAVE" headers

        // Find the format chunk
        while offset < buffer.len() - 8 {
            let chunk_id = &buffer[offset..offset + 4];
            if chunk_id == b"fmt " {
                format_chunk_offset = Some(offset + 8);
                // format_chunk_size = u32::from_le_bytes(buffer[offset + 4..offset + 8].try_into().unwrap());
            } else if chunk_id == b"data" {
                // data_chunk_offset = Some(offset + 8);
                let bytes = buffer[offset + 4..offset + 8].try_into();
                if let Ok(bytes) = bytes {
                    data_chunk_size = u32::from_le_bytes(bytes);
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Invalid data chunk",
                    ));
                }
            }
            let bytes = buffer[offset + 4..offset + 8].try_into();
            if let Ok(bytes) = bytes {
                let chunk_size = u32::from_le_bytes(bytes);
                offset += 8 + chunk_size as usize;
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid chunk size",
                ));
            }
        }

        let Some(format_chunk_offset) = format_chunk_offset else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Format chunk not found",
            ));
        };

        let format_chunk = FormatChunk::from_buffer(&buffer[format_chunk_offset..])?;

        // Get the number of channels from the format chunk
        let num_channels = format_chunk.num_channels;
        let block_align = format_chunk.block_align as usize;
        let num_samples = data_chunk_size as usize / block_align;
        #[allow(clippy::cast_precision_loss, clippy::cast_lossless)]
        let duration = num_samples as f64 / format_chunk.sample_rate as f64;

        Ok(WaveData {
            format: format_chunk,
            num_samples,
            duration,
            num_channels,
        })
    }
}
