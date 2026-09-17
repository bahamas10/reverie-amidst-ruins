/*!
 * audio engine
 */

use std::io::Cursor;

use anyhow::{Context, Result, bail};
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};

const REVERIE: &[u8] =
    include_bytes!("../assets/songs/reverie-amidst-ruins.ogg");
const ECHOES: &[u8] = include_bytes!("../assets/songs/echoes-of-solace.ogg");
const COLORS: &[u8] = include_bytes!("../assets/songs/colors.ogg");
const FINAL_STORM: &[u8] =
    include_bytes!("../assets/songs/the-final-storm.ogg");

pub struct Audio {
    _sink: MixerDeviceSink,
    music: Player,
}

impl Audio {
    pub fn new(initial_music: &str) -> Result<Self> {
        let mut sink = DeviceSinkBuilder::open_default_sink()
            .context("could not open the default audio output")?;
        sink.log_on_drop(false);

        let music = Player::connect_new(sink.mixer());
        music.set_volume(1.0);

        let audio = Self { _sink: sink, music };
        audio.set_music(initial_music)?;

        Ok(audio)
    }

    pub fn set_music(&self, name: &str) -> Result<()> {
        let bytes = match name {
            "reverie-amidst-ruins" => REVERIE,
            "echoes-of-solace" => ECHOES,
            "colors" => COLORS,
            "the-final-storm" => FINAL_STORM,
            _ => bail!("unknown music track: {name}"),
        };

        let source = Decoder::new_looped(Cursor::new(bytes))
            .with_context(|| format!("could not play music: {name}"))?;

        self.music.clear();
        self.music.append(source);
        self.music.play();

        Ok(())
    }
}
