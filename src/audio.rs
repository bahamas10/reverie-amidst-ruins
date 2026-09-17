/*!
 * audio engine
 */

use std::io::Cursor;

use anyhow::{Context, Result, bail};
use rodio::{
    Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source,
    buffer::SamplesBuffer,
};

const REVERIE: &[u8] =
    include_bytes!("../assets/songs/reverie-amidst-ruins.ogg");
const ECHOES: &[u8] = include_bytes!("../assets/songs/echoes-of-solace.ogg");
const COLORS: &[u8] = include_bytes!("../assets/songs/colors.ogg");
const FINAL_STORM: &[u8] =
    include_bytes!("../assets/songs/the-final-storm.ogg");
const JUMP: &[u8] = include_bytes!("../assets/sfx/jump.wav");
const COIN: &[u8] = include_bytes!("../assets/sfx/coin.wav");
const DEAD: &[u8] = include_bytes!("../assets/sfx/dead.wav");

pub enum SoundEffect {
    Jump,
    Coin,
    Dead,
}

pub struct Audio {
    sink: MixerDeviceSink,
    music: Player,

    jump: SamplesBuffer,
    coin: SamplesBuffer,
    dead: SamplesBuffer,
}

impl Audio {
    pub fn new(initial_music: &str) -> Result<Self> {
        let mut sink = DeviceSinkBuilder::open_default_sink()
            .context("could not open the default audio output")?;
        sink.log_on_drop(false);

        let music = Player::connect_new(sink.mixer());
        music.set_volume(1.0);

        // the sound effects felt laggy when i first introduced them, so cache
        // them here at startup
        let audio = Self {
            sink,
            music,
            jump: decode_effect(JUMP, "jump")?,
            coin: decode_effect(COIN, "coin")?,
            dead: decode_effect(DEAD, "dead")?,
        };

        audio.play_music(initial_music)?;

        Ok(audio)
    }

    pub fn play_music(&self, name: &str) -> Result<()> {
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

    pub fn play_effect(&self, effect: SoundEffect) -> Result<()> {
        let sound = match effect {
            SoundEffect::Jump => self.jump.clone(),
            SoundEffect::Coin => self.coin.clone(),
            SoundEffect::Dead => self.dead.clone(),
        };

        self.sink.mixer().add(sound);

        Ok(())
    }
}

fn decode_effect(bytes: &'static [u8], name: &str) -> Result<SamplesBuffer> {
    let source = Decoder::try_from(Cursor::new(bytes))
        .with_context(|| format!("could not decode sound effect: {name}"))?;

    let channels = source.channels();
    let sample_rate = source.sample_rate();
    let samples = source.collect::<Vec<_>>();

    Ok(SamplesBuffer::new(channels, sample_rate, samples))
}
