use anyhow::{anyhow, Result};
use serde::Serialize;
use std::io::Read;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NbtNode {
    pub name: String,
    pub tag: String,
    pub value: Option<String>,
    pub children: Vec<NbtNode>,
}

struct Cursor<'a> {
    b: &'a [u8],
    pos: usize,
}

impl Cursor<'_> {
    fn u8(&mut self) -> Result<u8> {
        let v = *self
            .b
            .get(self.pos)
            .ok_or_else(|| anyhow!("unexpected end of NBT"))?;
        self.pos += 1;
        Ok(v)
    }
    fn take(&mut self, n: usize) -> Result<&[u8]> {
        if self.pos + n > self.b.len() {
            return Err(anyhow!("unexpected end of NBT"));
        }
        let s = &self.b[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }
    fn u16(&mut self) -> Result<u16> {
        let s = self.take(2)?;
        Ok(u16::from_be_bytes([s[0], s[1]]))
    }
    fn i16(&mut self) -> Result<i16> {
        let s = self.take(2)?;
        Ok(i16::from_be_bytes([s[0], s[1]]))
    }
    fn i32(&mut self) -> Result<i32> {
        let s = self.take(4)?;
        Ok(i32::from_be_bytes([s[0], s[1], s[2], s[3]]))
    }
    fn i64(&mut self) -> Result<i64> {
        let s: [u8; 8] = self.take(8)?.try_into().unwrap();
        Ok(i64::from_be_bytes(s))
    }
    fn f32v(&mut self) -> Result<f32> {
        let s: [u8; 4] = self.take(4)?.try_into().unwrap();
        Ok(f32::from_be_bytes(s))
    }
    fn f64v(&mut self) -> Result<f64> {
        let s: [u8; 8] = self.take(8)?.try_into().unwrap();
        Ok(f64::from_be_bytes(s))
    }
    fn name(&mut self) -> Result<String> {
        let n = self.u16()? as usize;
        let s = self.take(n)?;
        Ok(String::from_utf8_lossy(s).into_owned())
    }
}

fn tag_name(t: u8) -> &'static str {
    match t {
        1 => "byte",
        2 => "short",
        3 => "int",
        4 => "long",
        5 => "float",
        6 => "double",
        7 => "byteArray",
        8 => "string",
        9 => "list",
        10 => "compound",
        11 => "intArray",
        12 => "longArray",
        _ => "end",
    }
}

fn preview<T: ToString>(len: usize, vals: &[T]) -> String {
    let head: Vec<String> =
        vals.iter().take(12).map(|x| x.to_string()).collect();
    if len > head.len() {
        format!("[{len}] {} …", head.join(", "))
    } else {
        format!("[{len}] {}", head.join(", "))
    }
}

fn read_payload(
    c: &mut Cursor,
    t: u8,
    name: String,
    depth: u32,
) -> Result<NbtNode> {
    if depth > 512 {
        return Err(anyhow!("NBT nesting too deep"));
    }
    let mut node = NbtNode {
        name,
        tag: tag_name(t).into(),
        value: None,
        children: Vec::new(),
    };
    match t {
        1 => node.value = Some((c.u8()? as i8).to_string()),
        2 => node.value = Some(c.i16()?.to_string()),
        3 => node.value = Some(c.i32()?.to_string()),
        4 => node.value = Some(c.i64()?.to_string()),
        5 => node.value = Some(c.f32v()?.to_string()),
        6 => node.value = Some(c.f64v()?.to_string()),
        7 => {
            let n = c.i32()?.max(0) as usize;
            let s = c.take(n)?;
            let vals: Vec<i8> = s.iter().take(12).map(|b| *b as i8).collect();
            node.value = Some(preview(n, &vals));
        }
        8 => {
            let n = c.u16()? as usize;
            let s = c.take(n)?;
            node.value = Some(String::from_utf8_lossy(s).into_owned());
        }
        9 => {
            let child_t = c.u8()?;
            let n = c.i32()?.max(0) as usize;
            for i in 0..n {
                node.children.push(read_payload(
                    c,
                    child_t,
                    i.to_string(),
                    depth + 1,
                )?);
            }
            node.value = Some(format!("{n} × {}", tag_name(child_t)));
        }
        10 => loop {
            let ct = c.u8()?;
            if ct == 0 {
                break;
            }
            let cn = c.name()?;
            node.children.push(read_payload(c, ct, cn, depth + 1)?);
        },
        11 => {
            let n = c.i32()?.max(0) as usize;
            let mut vals = Vec::with_capacity(n.min(12));
            for i in 0..n {
                let v = c.i32()?;
                if i < 12 {
                    vals.push(v);
                }
            }
            node.value = Some(preview(n, &vals));
        }
        12 => {
            let n = c.i32()?.max(0) as usize;
            let mut vals = Vec::with_capacity(n.min(12));
            for i in 0..n {
                let v = c.i64()?;
                if i < 12 {
                    vals.push(v);
                }
            }
            node.value = Some(preview(n, &vals));
        }
        _ => {}
    }
    Ok(node)
}

fn decompress(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
        flate2::read::GzDecoder::new(bytes).read_to_end(&mut out)?;
        Ok(out)
    } else if bytes.first() == Some(&0x78) {
        flate2::read::ZlibDecoder::new(bytes).read_to_end(&mut out)?;
        Ok(out)
    } else {
        Ok(bytes.to_vec())
    }
}

pub fn parse(bytes: &[u8]) -> Result<NbtNode> {
    let data = decompress(bytes)?;
    let mut c = Cursor { b: &data, pos: 0 };
    let t = c.u8()?;
    if t == 0 || t > 12 {
        return Err(anyhow!("not an NBT file"));
    }
    let root_name = c.name()?;
    read_payload(&mut c, t, root_name, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn sample() -> Vec<u8> {
        vec![
            0x0a, 0x00, 0x00, 0x03, 0x00, 0x01, b'x', 0x00, 0x00, 0x00, 0x2a,
            0x00,
        ]
    }

    #[test]
    fn parses_raw_compound() {
        let n = parse(&sample()).unwrap();
        assert_eq!(n.tag, "compound");
        assert_eq!(n.children.len(), 1);
        assert_eq!(n.children[0].tag, "int");
        assert_eq!(n.children[0].name, "x");
        assert_eq!(n.children[0].value.as_deref(), Some("42"));
    }

    #[test]
    fn parses_gzip_compressed() {
        let mut e = flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::default(),
        );
        e.write_all(&sample()).unwrap();
        let gz = e.finish().unwrap();
        let n = parse(&gz).unwrap();
        assert_eq!(n.children[0].value.as_deref(), Some("42"));
    }

    #[test]
    fn rejects_non_nbt() {
        assert!(parse(&[0xff, 0xff, 0xff]).is_err());
        assert!(parse(&[]).is_err());
    }
}
