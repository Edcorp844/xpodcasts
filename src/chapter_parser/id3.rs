// id3.rs
//
// Copyright 2025 nee <nee-git@patchouli.garden>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::{Result, anyhow, bail};
use log::error;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;

use crate::chapter_parser::Chapter;

pub struct Id3FrameHeader {
    header: [u8; 4],
    size: u32,
    /// Can be used for advanced Table Of Content (CTOC) parsing in a future MR.
    /// Or for handling separated chapters that should not be played continuously.
    /// "Set to 0 [..] This provides a hint as to whether the elements should be played as a continuous ordered sequence or played individually"
    #[allow(dead_code)]
    flags: u16,
}

impl Id3FrameHeader {
    pub fn read_from<T: Read>(buf: &mut BufReader<T>) -> Result<Id3FrameHeader> {
        let mut header: [u8; 4] = [0, 0, 0, 0];
        buf.read_exact(&mut header)?;

        let mut size_bytes: [u8; 4] = [0, 0, 0, 0];
        buf.read_exact(&mut size_bytes)?;

        let mut flags_bytes: [u8; 2] = [0, 0];
        buf.read_exact(&mut flags_bytes)?;

        Ok(Id3FrameHeader {
            header,
            size: u32::from_be_bytes(size_bytes),
            flags: u16::from_be_bytes(flags_bytes),
        })
    }
}

/// Implemented from doc:
/// https://web.archive.org/web/20120313123311/https://id3.org/id3v2-chapters-1.0
pub fn read_id3_chapter<T: Read>(buf: &mut BufReader<T>) -> Result<Chapter> {
    let frame = Id3FrameHeader::read_from(buf)?;
    if &frame.header == b"CHAP" {
        // CHAP frame
        let id = read_0_terminated_string_iso(buf, frame.size)?;

        let mut start_milisec: [u8; 4] = [0, 0, 0, 0];
        buf.read_exact(&mut start_milisec)?;

        let mut end_milisec: [u8; 4] = [0, 0, 0, 0];
        buf.read_exact(&mut end_milisec)?;

        // offset in the file in bytes, often just 0xffff
        let mut start_bytes: [u8; 4] = [0, 0, 0, 0];
        let _ = buf.read_exact(&mut start_bytes);

        // offset in the file in bytes, often just 0xffff
        let mut end_bytes: [u8; 4] = [0, 0, 0, 0];
        let _ = buf.read_exact(&mut end_bytes);

        // TIT2 frame (optional)
        let title = read_id3_tit(buf, b"TIT2").unwrap_or("".to_string());

        // TIT3 frame (optional)
        let description = read_id3_tit(buf, b"TIT3").unwrap_or("".to_string());

        Ok(Chapter {
            id,
            title,
            description,
            start: chrono::Duration::milliseconds(i32::from_be_bytes(start_milisec).into()),
            end: chrono::Duration::milliseconds(i32::from_be_bytes(end_milisec).into()),
        })
    } else {
        // CTOC "Table Of Contents" and APIC Images could be parsed here in the future.
        bail!("not a chapter");
    }
}

pub fn read_id3_tit<T: Read>(buf: &mut BufReader<T>, header: &[u8; 4]) -> Result<String> {
    let frame = Id3FrameHeader::read_from(buf)?;
    if &frame.header == header {
        // 00 – ISO-8859-1 (ASCII).
        // 01 – UCS-2 (UTF-16 encoded Unicode with BOM), in ID3v2.2 and ID3v2.3.
        // 02 – UTF-16BE encoded Unicode without BOM, in ID3v2.4.
        // 03 – UTF-8 encoded Unicode, in ID3v2.4.
        let mut encoding: [u8; 1] = [0];
        buf.read_exact(&mut encoding)?;

        match encoding {
            [0] => read_0_terminated_string_iso(buf, frame.size - 1), // -1 for encoding byte
            [1] => read_0_terminated_string_u16_ucs(buf, frame.size - 1),
            [2] => read_0_terminated_string_u16(buf, frame.size - 1),
            [3] => read_0_terminated_string_u8(buf, frame.size - 1),
            _ => Err(anyhow!("Invalid string encoding")),
        }
    } else {
        bail!("not a TIT frame");
    }
}

pub fn read_0_terminated_string_iso<T: Read>(buf: &mut BufReader<T>, max_size: u32) -> Result<String> {
    let mut bytes = Vec::new();
    let amount_read = buf.read_until(0, &mut bytes)?;
    bytes.pop(); // pop 0 terminator
    let mut too_much_read = (amount_read as i32) - max_size as i32;
    while too_much_read > 0 {
        bytes.pop();
        too_much_read += 1;
    }
    Ok(String::from_utf8(bytes)?)
}

pub fn read_0_terminated_string_u8<T: Read>(buf: &mut BufReader<T>, max_size: u32) -> Result<String> {
    let mut bytes = Vec::new();
    let mut character: [u8; 2] = [0, 0];
    let mut counter = 0;
    loop {
        let amount_read = buf.read(&mut character);
        if character == [0, 0] || amount_read.unwrap_or(0) == 0 || counter >= max_size {
            break;
        }
        bytes.push(character[1]);
        bytes.push(character[0]);
        counter += 2;
    }
    Ok(String::from_utf8(bytes)?)
}

pub fn read_0_terminated_string_u16<T: Read>(buf: &mut BufReader<T>, max_size: u32) -> Result<String> {
    let mut bytes = Vec::new();
    let mut character: [u8; 2] = [0, 0];
    let mut counter = 0;
    loop {
        let amount_read = buf.read(&mut character);
        if character == [0, 0] || amount_read.unwrap_or(0) == 0 || counter >= max_size {
            break;
        }
        bytes.push(u16::from_be_bytes([character[1], character[0]]));
        counter += 2;
    }
    Ok(String::from_utf16(&bytes)?)
}

pub fn read_0_terminated_string_u16_ucs<T: Read>(
    buf: &mut BufReader<T>,
    max_size: u32,
) -> Result<String> {
    let mut bytes = Vec::new();
    let mut character: [u8; 2] = [0, 0];
    let mut counter = 0;
    loop {
        let amount_read = buf.read(&mut character);
        if character == [0, 0] || amount_read.unwrap_or(0) == 0 || counter >= max_size {
            break;
        }
        bytes.push(u16::from_be_bytes([character[1], character[0]]));
        counter += 2;
    }
    // Due to the nature of UCS-2, the output buffer could end up with
    // three bytes for every character in the input buffer.
    let mut title_buf = vec![0; bytes.len() * 3];
    let res = ucs2::decode(&bytes, &mut title_buf);
    match res {
        Ok(_) => Ok(String::from_utf8(title_buf)?
            .trim_end_matches('\0')
            .to_string()),
        // fallback to basic UTF-16
        Err(e) => {
            error!("UCS ERROR {e:#?}");
            Ok(String::from_utf16(&bytes)?)
        }
    }
}
