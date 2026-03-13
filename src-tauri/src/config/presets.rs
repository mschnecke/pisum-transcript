//! Built-in preset definitions

use super::schema::Preset;

pub fn get_builtin_presets() -> Vec<Preset> {
    vec![
        Preset {
            id: "de-transcribe".to_string(),
            name: "Transcribe DE".to_string(),
            system_prompt: "Transcribe the following German audio accurately. \
                Output only the transcription without any additional commentary."
                .to_string(),
            is_builtin: true,
        },
        Preset {
            id: "en-transcribe".to_string(),
            name: "Transcribe EN".to_string(),
            system_prompt: "Transcribe the following English audio accurately. \
                Output only the transcription without any additional commentary."
                .to_string(),
            is_builtin: true,
        },
    ]
}
