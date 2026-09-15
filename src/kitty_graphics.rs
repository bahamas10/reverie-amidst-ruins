/*!
 * Functions to use the kitty graphics protocol but simplified for just the
 * subset of stuff I need.
 *
 * Based on https://sw.kovidgoyal.net/kitty/graphics-protocol/
 */

use std::io::Write;

use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use crossterm::{cursor, execute, terminal};
use flate2::{Compression, write::ZlibEncoder};

const APC_START: &[u8] = b"\x1b_G";
const APC_END: &[u8] = b"\x1b\\";
const MAX_PAYLOAD_SIZE: usize = 4096;
const IMAGE_ID: u32 = 1;
const PLACEMENT_ID: u32 = 1;

/// Displays raw RGBA pixels at the home location
pub fn display_rgba(
    data: &[u8],
    width: usize,
    height: usize,
    display_width: u16,
) -> Result<()> {
    let mut stdout = std::io::stdout().lock();

    execute!(stdout, cursor::MoveTo(0, 0))?;
    write_rgba(&mut stdout, data, width, height, display_width)?;

    stdout.flush()?;

    Ok(())
}

fn write_rgba(
    output: &mut impl Write,
    data: &[u8],
    width: usize,
    height: usize,
    display_width: u16,
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
            // A=T: transfer and display image
            // f=32: 32bit RGBA data
            // s=width: image width
            // v=height: image height
            // c=display_width: width in terminal columns
            // i/p: replace the same image placement every frame
            // o=z: image data is compressed with zlib
            // C=1: don't move the cursor
            // q=2: quiet errors
            write!(
                output,
                "a=T,f=32,s={width},v={height},c={display_width},i={IMAGE_ID},p={PLACEMENT_ID},o=z,C=1,q=2,"
            )?;
            first = false;
        }

        // check if we have more: m=0 if done, m=1 if more
        let have_more = chunks.peek().is_some();
        let m = match have_more {
            true => 1,
            false => 0,
        };
        write!(output, "m={};", m)?;

        output.write_all(chunk)?;
        output.write_all(APC_END)?;
    }

    Ok(())
}
