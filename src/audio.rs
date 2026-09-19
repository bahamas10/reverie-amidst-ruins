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
const MUSIC_VOLUME: f32 = 1.0;
const SFX_VOLUME: f32 = 0.55;

pub enum SoundEffect {
    Jump,
    Coin,
    Dead,
}

pub struct Audio {
    sink: MixerDeviceSink,
    music: Player,
    muted: bool,
    volume: u8,

    jump: SamplesBuffer,
    coin: SamplesBuffer,
    dead: SamplesBuffer,
}

impl Audio {
    pub fn new(initial_music: &str, volume: u8) -> Result<Self> {
        let mut sink = DeviceSinkBuilder::open_default_sink()
            .context("could not open the default audio output")?;
        sink.log_on_drop(false);

        let music = Player::connect_new(sink.mixer());
        let volume = volume.min(100);
        music.set_volume(MUSIC_VOLUME * volume as f32 / 100.0);

        // the sound effects felt laggy when i first introduced them, so cache
        // them here at startup
        let audio = Self {
            sink,
            music,
            muted: false,
            volume,
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
        let volume = self.volume_scale();
        if volume == 0.0 {
            return Ok(());
        }

        let sound = match effect {
            SoundEffect::Jump => self.jump.clone(),
            SoundEffect::Coin => self.coin.clone(),
            SoundEffect::Dead => self.dead.clone(),
        };

        self.sink.mixer().add(sound.amplify(SFX_VOLUME * volume));

        Ok(())
    }

    /// Toggle all game audio, returning whether it is now muted.
    pub fn toggle_mute(&mut self) -> bool {
        self.muted = !self.muted;
        self.update_music_volume();
        self.muted
    }

    /// Adjust master volume, returning its new 0-100 value.
    pub fn adjust_volume(&mut self, amount: i8) -> u8 {
        self.volume = (self.volume as i16 + amount as i16).clamp(0, 100) as u8;
        self.update_music_volume();
        self.volume
    }

    fn update_music_volume(&self) {
        self.music.set_volume(MUSIC_VOLUME * self.volume_scale());
    }

    fn volume_scale(&self) -> f32 {
        if self.muted { 0.0 } else { self.volume as f32 / 100.0 }
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
