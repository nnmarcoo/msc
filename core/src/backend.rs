use kira::{AudioManager, AudioManagerSettings, DefaultBackend, sound::{FromFileError, streaming::StreamingSoundHandle}};

pub struct Backend {
    pub manager: AudioManager,
    pub sound: Option<StreamingSoundHandle<FromFileError>>
}


impl Backend {
    pub fn new() -> Self {
        Backend {
            manager: AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap(),
            sound: None,
        }
    }

}
