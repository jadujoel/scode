use std::io::{self, Cursor, Read};
use byteorder::{LittleEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FormatChunk {
    pub audio_format: u16,
    pub num_channels: u16,
    pub sample_rate: u32,
    pub byte_rate: u32,
    pub block_align: u16,
    pub bits_per_sample: u16,
}

impl FormatChunk {
    fn from_buffer(cursor: &mut Cursor<&[u8]>) -> io::Result<Self> {
        Ok(FormatChunk {
            audio_format: cursor.read_u16::<LittleEndian>()?,
            num_channels: cursor.read_u16::<LittleEndian>()?,
            sample_rate: cursor.read_u32::<LittleEndian>()?,
            byte_rate: cursor.read_u32::<LittleEndian>()?,
            block_align: cursor.read_u16::<LittleEndian>()?,
            bits_per_sample: cursor.read_u16::<LittleEndian>()?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Data {
    pub format: FormatChunk,
    pub num_samples: usize,
    pub duration: f64,
}

impl Data {
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
        let mut format = None;
        let mut data_chunk_size = 0u32;

        let mut cursor = Cursor::new(buffer);
        cursor.set_position(12); // Skip "RIFF" and "WAVE" headers

        let mut found_data_chunk = false;
        if buffer.len() < 36 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Buffer is too small",
            ));
        }
        while (cursor.position() as usize) < buffer.len() - 8 {
            let mut chunk_id = [0u8; 4];
            cursor.read_exact(&mut chunk_id)?;

            let chunk_size = cursor.read_u32::<LittleEndian>()?;
            match &chunk_id {
                b"fmt " => {
                    format = Some(FormatChunk::from_buffer(&mut cursor)?);
                    match &format {
                        Some(f) => {
                            if f.audio_format != 1 {
                                return Err(io::Error::new(
                                    io::ErrorKind::InvalidData,
                                    format!("Audio format {} is not PCM", f.audio_format),
                                ));
                            }
                        }
                        None => {
                            return Err(io::Error::new(
                                io::ErrorKind::NotFound,
                                "Format chunk not found",
                            ));
                        }
                    }
                }
                b"data" => {
                    found_data_chunk = true;
                    data_chunk_size = chunk_size;
                    break;
                }
                _ => {
                    // Skip over the chunk's content if it's not "fmt " or "data"
                   cursor.set_position(cursor.position() + u64::from(chunk_size));
                }
            }
        }
        if !found_data_chunk {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Data chunk not found",
            ));
        }
        let format = format.ok_or(io::Error::new(
            io::ErrorKind::NotFound,
            "Format chunk not found",
        ))?;
        let duration = f64::from(data_chunk_size) / f64::from(format.byte_rate);
        let num_samples = data_chunk_size as usize / format.block_align as usize;
        if num_samples == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Number of samples is zero",
            ));
        }
        if (num_samples * format.block_align as usize) as u32 != data_chunk_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Data chunk size is invalid",
            ));
        }
        Ok(Data {
            format,
            num_samples,
            duration
        })
    }
}

// #[derive(Debug, Clone, Copy)]
// pub enum Format {
//     Unknown = 0,
//     Pcm = 1,
//     Adpcm = 2,
//     Float = 3,
//     VSELP = 4,
//     IBM_CVSD = 5,
//     Alaw = 6,
//     Mulaw = 7,
//     DTS = 8,
//     DRM = 9,
//     WMAVoice9 = 10,
//     WMAVoice10 = 11,
//     OKI_ADPCM = 16,
//     DVI_ADPCM = 17,
//     // IMA_ADPCM = DVI_ADPCM, // Alias
//     Mediaspace_ADPCM = 18,
//     Sierra_ADPCM = 19,
//     G723_ADPCM = 20,
//     DIGISTD = 21,
//     DIGIFIX = 22,
//     Dialogic_OKI_ADPCM = 23,
//     MediaVision_ADPCM = 24,
//     CU_CODEC = 25,
//     HP_DYN_VOICE = 26,
//     Yamaha_ADPCM = 32,
//     SONARC = 33,
//     DSPGroup_TrueSpeech = 34,
//     EchoSpeech_Corporation1 = 35,
//     AudioFile_AF36 = 36,
//     APTX = 37,
//     AudioFile_AF10 = 38,
//     Prosody_1612 = 39,
//     LRC = 40,
//     Dolby_AC2 = 48,
//     GSM610 = 49,
//     MSNAudio = 50,
//     ANTEX_ADPCME = 51,
//     Control_Res_VQLPC = 52,
//     DIGIREAL = 53,
//     DIGIADPCM = 54,
//     Control_Res_CR10 = 55,
//     NMS_VBXADPCM = 56,
//     CS_IMAADPCM = 57,
//     EchoSC3 = 58,
//     Rockwell_ADPCM = 59,
//     Rockwell_DigitalK = 60,
//     Xebec = 61,
//     G721_ADPCM = 64,
//     G728_CELP = 65,
//     MSG723 = 66,
//     Intel_G723_1 = 67,
//     Intel_G729 = 68,
//     Sharp_G726 = 69,
//     MPEG = 80,
//     RT24 = 82,
//     PAC = 83,
//     MPEGLayer3 = 85,
//     Lucent_G723 = 89,
//     Cirrus = 96,
//     ESSPCM = 97,
//     Voxware = 98,
//     Canopus_ATRAC = 99,
//     G726_ADPCM = 100,
//     G722_ADPCM = 101,
//     DSAT = 102,
//     DSAT_DISPLAY = 103,
//     VoxwareByteAligned = 105,
//     VoxwareAC8 = 112,
//     VoxwareAC10 = 113,
//     VoxwareAC16 = 114,
//     VoxwareAC20 = 115,
//     VoxwareMetaVoice = 116,
//     VoxwareMetaSound = 117,
//     VoxwareRT29HW = 118,
//     VoxwareVR12 = 119,
//     VoxwareVR18 = 120,
//     VoxwareTQ40 = 121,
//     VoxwareSC3 = 122,
//     VoxwareSC3_1 = 123,
//     Softsound = 128,
//     VoxwareTQ60 = 129,
//     MSRT24 = 130,
//     G729A = 131,
//     MVI_MVI2 = 132,
//     DF_G726 = 133,
//     DF_GSM610 = 134,
//     ISIAudio = 136,
//     Onlive = 137,
//     Multitude_FT_SX20 = 138,
//     INFOCOM_ITS_G721_ADPCM = 139,
//     Convedia_G729 = 140,
//     Congruency = 141,
//     SBC24 = 145,
//     Dolby_AC3_SPDIF = 146,
//     MediaSonic_G723 = 147,
//     Prosody_8KBPS = 148,
//     ZYXEL_ADPCM = 151,
//     Philips_LPCBB = 152,
//     Packed = 153,
//     Malden_PhonyTalk = 160,
//     Racal_Recorder_GSM = 161,
//     Racal_Recorder_G720_A = 162,
//     Racal_Recorder_G723_1 = 163,
//     Racal_Recorder_TETRA_ACELP = 164,
//     NEC_AAC = 176,
//     Raw_AAC1 = 255,
//     Rhetorex_ADPCM = 256,
//     IRAT = 257,
//     Vivo_G723 = 273,
//     Vivo_Siren = 274,
//     Philips_Celp = 288,
//     Philips_GRUNDIG = 289,
//     Digital_G723 = 290,
//     Sanyo_LD_ADPCM = 291,
//     Siprolab_ACEPLNET = 304,
//     Siprolab_ACELP4800 = 305,
//     Siprolab_ACELP8V3 = 306,
//     Siprolab_G729 = 307,
//     Siprolab_G729A = 308,
//     Siprolab_Kelvin = 309,
//     VoiceAgeAMR = 310,
//     G726ADPCM = 320,
//     Dictaphone_CELP68 = 321,
//     Dictaphone_CELP54 = 322,
//     Qualcomm_PureVoice = 336,
//     Qualcomm_HalfRate = 337,
//     TubGSM = 341,
//     MSAudio1 = 352,
//     WMAudio2 = 353,
//     WMAudio3 = 354,
//     WMAudioLossless = 355,
//     WMASPDIF = 356,
//     Unisys_NAP_ADPCM = 368,
//     Unisys_NAP_ULAW = 369,
//     Unisys_NAP_ALAW = 370,
//     Unisys_NAP_16K = 371,
//     SYCOM_ACM_SYC008 = 372,
//     SYCOM_ACM_SYC701_G726L = 373,
//     SYCOM_ACM_SYC701_CELP54 = 374,
//     SYCOM_ACM_SYC701_CELP68 = 375,
//     KnowledgeAdventure_ADPCM = 376,
//     Fraunhofer_IIS_MPEG2_AAC = 384,
//     DTS_DS = 400,
//     Creative_ADPCM = 512,
//     Creative_Fastspeech8 = 514,
//     Creative_Fastspeech10 = 515,
//     UHER_ADPCM = 528,
//     Ulead_DV_AUDIO = 533,
//     Ulead_DV_AUDIO_1 = 534,
//     Quarterdeck = 544,
//     ILink_VC = 560,
//     RawSport = 576,
//     ESST_AC3 = 577,
//     GenericPassthru = 585,
//     IPI_HSX = 592,
//     IPI_RPELP = 593,
//     CS2 = 608,
//     Sony_SCX = 624,
//     Sony_SCY = 625,
//     Sony_ATRAC3 = 626,
//     Sony_SPC = 627,
//     TELUM_AUDIO = 640,
//     TELUM_IA_AUDIO = 641,
//     NORCOM_VOICE_SYSTEMS_ADPCM = 645,
//     FM_TOWNS_SND = 768,
//     Micronas = 848,
//     Micronas_CELP833 = 849,
//     BTV_DIGITAL = 1024,
//     Intel_MusicCoder = 1025,
//     Indeo_Audio = 1026,
//     QDesign_Music = 1104,
//     On2_VP7_Audio = 1280,
//     On2_VP6_Audio = 1281,
//     VME_VMPCM = 1664,
//     TPC = 1665,
//     Lightwave_Lossless = 2222,
//     Olivetti_GSM = 4096,
//     Olivetti_ADPCM = 4097,
//     Olivetti_CELP = 4098,
//     Olivetti_SBC = 4099,
//     Olivetti_OPR = 4100,
//     LH_CODEC = 4352,
//     LH_CODEC_CELP = 4353,
//     LH_CODEC_SBC8 = 4354,
//     LH_CODEC_SBC12 = 4355,
//     LH_CODEC_SBC16 = 4356,
//     Norris = 5120,
//     ISIAudio2 = 5121,
//     SoundSpaceMusicompress = 5376,
//     MPEG_ADTS_AAC = 5632,
//     MPEG_RAW_AAC = 5633,
//     MPEG_LOAS = 5634,
//     Nokia_MPEG_ADTS_AAC = 5640,
//     Nokia_MPEG_RAW_AAC = 5641,
//     Vodafone_MPEG_ADTS_AAC = 5642,
//     Vodafone_MPEG_RAW_AAC = 5643,
//     MPEG_HEAAC = 5648,
//     Voxware_RT24_Speech = 6172,
//     SonicFoundry_LOSSLESS = 6513,
//     Innings_Telecom_ADPCM = 6521,
//     Lucent_SX8300P = 7175,
//     Lucent_SX5363S = 7180,
//     CUSEEME = 7939,
//     NTCSOFT_ALF2CM_ACM = 8132,
//     DVM = 8192,
//     DTS2 = 8193,
//     MAKEAVIS = 13075,
//     Divio_MPEG4_AAC = 16707,
//     Nokia_Adaptive_Multirate = 16897,
//     Divio_G726 = 16963,
//     LEAD_Speech = 17228,
//     LEAD_Vorbis = 22092,
//     WavPack_Audio = 22358,
//     OGG_Vorbis_Mode1 = 26447,
//     OGG_Vorbis_Mode2 = 26448,
//     OGG_Vorbis_Mode3 = 26449,
//     OGG_Vorbis_Mode1Plus = 26479,
//     OGG_Vorbis_Mode2Plus = 26480,
//     OGG_Vorbis_Mode3Plus = 26481,
//     ThreeCOM_NBX = 28672,
//     FAAD_AAC = 28781,
//     AMR_NB = 29537,
//     AMR_WB = 29538,
//     AMR_WP = 29539,
//     GSM_AMR_CBR = 31265,
//     GSM_AMR_VBR_SID = 31266,
//     Comverse_Infosys_G723_1 = 41216,
//     Comverse_Infosys_Avqsbc = 41217,
//     Comverse_Infosys_SBC = 41218,
//     Symbol_G729A = 41219,
//     VoiceAgeAMRWB = 41220,
//     Ingenient_G726 = 41221,
//     MPEG4_AAC = 41222,
//     Encore_G726 = 41223,
//     ZOLL_ASAO = 41224,
//     Speex_Voice = 41225,
//     Vianix_Masc = 41226,
//     WM9_Spectrum_Analyzer = 41227,
//     WMF_Spectrum_Anayzer = 41228,
//     GSM_610 = 41229,
//     GSM_620 = 41230,
//     GSM_660 = 41231,
//     GSM_690 = 41232,
//     GSM_ADAPTIVE_MULTIRATE_WB = 41233,
//     Polycom_G722 = 41234,
//     Polycom_G728 = 41235,
//     Polycom_G729A = 41236,
//     Polycom_Siren = 41237,
//     Global_IP_ILBC = 41238,
//     Radiotime_TimeShiftRadio = 41239,
//     Nice_ACA = 41240,
//     Nice_ADPCM = 41241,
//     Vocord_G721 = 41242,
//     Vocord_G726 = 41243,
//     Vocord_G722_1 = 41244,
//     Vocord_G728 = 41245,
//     Vocord_G729 = 41246,
//     Vocord_G729A = 41247,
//     Vocord_G723_1 = 41248,
//     Vocord_LBC = 41249,
//     Nice_G728 = 41250,
//     France_Telecom_G729 = 41251,
//     Codian = 41252,
//     FLAC = 61868,
//     Extensible = 0xFFFE,
//     Development = 0xFFFF,
// }
