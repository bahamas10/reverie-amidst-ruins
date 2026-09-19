/*!
 * Functions to use the kitty graphics protocol but simplified for just the
 * subset of stuff I need.
 *
 * Based on https://sw.kovidgoyal.net/kitty/graphics-protocol/
 */

use std::io::Write;

use anyhow::Result;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use crossterm::{cursor, execute};
use flate2::{Compression, write::ZlibEncoder};

use super::ImagePlacement;

pub const APC_START: &[u8] = b"\x1b_G";
pub const APC_END: &[u8] = b"\x1b\\";
pub const MAX_PAYLOAD_SIZE: usize = 4096;
pub const IMAGE_ID: u32 = 1;
pub const PLACEMENT_ID: u32 = 1;

/// Displays raw RGBA pixels at the requested terminal placement
pub fn display_rgba(
    data: &[u8],
    width: usize,
    height: usize,
    placement: ImagePlacement,
) -> Result<()> {
    let mut stdout = std::io::stdout().lock();

    execute!(stdout, cursor::MoveTo(placement.column, placement.row))?;
    write_rgba(&mut stdout, data, width, height, placement)?;

    stdout.flush()?;

    Ok(())
}

fn write_rgba(
    output: &mut impl Write,
    data: &[u8],
    width: usize,
    height: usize,
    placement: ImagePlacement,
) -> Result<()> {
    // compress the image because omg it's slow to render
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(data)?;

    let encoded = STANDARD.encode(encoder.finish()?);
    let mut chunks = encoded.as_bytes().chunks(MAX_PAYLOAD_SIZE).peekable();

    let mut first = true;
    while let Some(chunk) = chunks.next() {
        output.write_all(APC_START)?;

        // print metadata for the first loop
        if first {
            let d_width = placement.width;
            let d_height = placement.height;
            write!(output, "a=T,")?; // transfer and display image
            write!(output, "f=32,")?; // 32bit RGBA data
            write!(output, "s={width},")?; // image width
            write!(output, "v={height},")?; // image height
            write!(output, "c={d_width},")?; // display width
            write!(output, "r={d_height},")?; // display height
            write!(output, "i={IMAGE_ID},")?; // re-use the same image id
            write!(output, "p={PLACEMENT_ID},")?; // re-use placement id
            write!(output, "o=z,")?; // zlib compression
            write!(output, "C=1,")?; // don't move the cursor
            write!(output, "z=-1,")?; // z-index below text
            first = false;
        }

        // check if we have more: m=0 if done, m=1 if more
        let have_more = chunks.peek().is_some();
        let m = match have_more {
            true => 1,
            false => 0,
        };
        // repeat q on continuation chunks too. kitty on iterm2 doesn't seem to
        // carry the quiet setting forward from the first chunk.
        write!(output, "m={},q=1;", m)?;

        output.write_all(chunk)?;
        output.write_all(APC_END)?;
    }

    Ok(())
}
