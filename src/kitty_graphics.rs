/*!
 * Functions to use the kitty graphics protocol but simplified for just the
 * subset of stuff I need.
 *
 * Based on https://sw.kovidgoyal.net/kitty/graphics-protocol/
 */

use std::io::{self, Write};

use anyhow::Result;
use base64::{Engine as _, engine::general_purpose::STANDARD};

const APC_START: &[u8] = b"\x1b_G";
const APC_END: &[u8] = b"\x1b\\";
const MAX_PAYLOAD_SIZE: usize = 4096;

/// Displays raw RGBA pixels at the cursor using to stdout
pub fn display_rgba(
    data: &[u8],
    width: usize,
    height: usize,
    display_width: u16,
) -> Result<()> {
    let mut stdout = io::stdout().lock();
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
    let encoded = STANDARD.encode(data);
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
            // q=2: quiet errors
            write!(
                output,
                "a=T,f=32,s={width},v={height},c={display_width},q=2,"
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
