//! Gera assets/halo.ico e assets/halo.png a partir do desenho em src/icon.rs.
//! `cargo run --example make_icon`
#[path = "../src/icon.rs"]
#[allow(dead_code)]
mod icon;

use std::io::Write;

/// Entrada de ICO em BMP 32 bits (BGRA de baixo para cima + máscara AND).
fn bmp_entry(size: u32) -> Vec<u8> {
    let rgba = icon::render(size, icon::Spec::APP);
    let mut out = Vec::new();
    let header: [u32; 10] = [40, size, size * 2, 0, 0, 0, 0, 0, 0, 0];
    out.extend_from_slice(&header[0].to_le_bytes());
    out.extend_from_slice(&(size as i32).to_le_bytes());
    out.extend_from_slice(&((size * 2) as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // planos
    out.extend_from_slice(&32u16.to_le_bytes()); // bits por pixel
    out.extend_from_slice(&[0u8; 24]); // compressão, tamanho, resolução, cores
    for y in (0..size).rev() {
        for x in 0..size {
            let i = ((y * size + x) * 4) as usize;
            out.extend_from_slice(&[rgba[i + 2], rgba[i + 1], rgba[i], rgba[i + 3]]);
        }
    }
    let mask_row = size.div_ceil(32) * 4;
    out.extend(std::iter::repeat_n(0u8, (mask_row * size) as usize));
    out
}

fn main() {
    let sizes = [16u32, 24, 32, 48, 64, 128, 256];
    let entries: Vec<Vec<u8>> = sizes.iter().map(|&s| bmp_entry(s)).collect();
    let mut ico = Vec::new();
    ico.extend_from_slice(&[0, 0, 1, 0]);
    ico.extend_from_slice(&(sizes.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * sizes.len() as u32;
    for (s, e) in sizes.iter().zip(&entries) {
        let dim = if *s >= 256 { 0 } else { *s as u8 };
        ico.extend_from_slice(&[dim, dim, 0, 0]);
        ico.extend_from_slice(&1u16.to_le_bytes());
        ico.extend_from_slice(&32u16.to_le_bytes());
        ico.extend_from_slice(&(e.len() as u32).to_le_bytes());
        ico.extend_from_slice(&offset.to_le_bytes());
        offset += e.len() as u32;
    }
    for e in &entries {
        ico.extend_from_slice(e);
    }
    std::fs::File::create("assets/halo.ico").and_then(|mut f| f.write_all(&ico)).expect("halo.ico");

    // PNG 256 para o README e o Linux: dados sem compressão (deflate "stored").
    let size = 256u32;
    let rgba = icon::render(size, icon::Spec::APP);
    let mut raw = Vec::new();
    for y in 0..size as usize {
        raw.push(0);
        raw.extend_from_slice(&rgba[y * size as usize * 4..(y + 1) * size as usize * 4]);
    }
    std::fs::write("assets/halo.png", png(size, &raw)).expect("halo.png");
    println!("assets/halo.ico e assets/halo.png gerados");
}

fn png(size: u32, raw: &[u8]) -> Vec<u8> {
    fn crc(data: &[u8]) -> u32 {
        let mut c = 0xFFFF_FFFFu32;
        for &b in data {
            c ^= b as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            }
        }
        !c
    }
    fn chunk(out: &mut Vec<u8>, kind: &[u8], data: &[u8]) {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend_from_slice(data);
        out.extend_from_slice(&body);
        out.extend_from_slice(&crc(&body).to_be_bytes());
    }
    let mut z = vec![0x78, 0x01];
    let chunks: Vec<&[u8]> = raw.chunks(65535).collect();
    for (i, c) in chunks.iter().enumerate() {
        z.push(u8::from(i == chunks.len() - 1));
        z.extend_from_slice(&(c.len() as u16).to_le_bytes());
        z.extend_from_slice(&(!(c.len() as u16)).to_le_bytes());
        z.extend_from_slice(c);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &x in raw {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&size.to_be_bytes());
    ihdr.extend_from_slice(&size.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &z);
    chunk(&mut out, b"IEND", &[]);
    out
}
