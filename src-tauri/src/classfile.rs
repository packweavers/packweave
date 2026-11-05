struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn u1(&mut self) -> Option<u8> {
        let b = *self.bytes.get(self.at)?;
        self.at += 1;
        Some(b)
    }

    fn u2(&mut self) -> Option<u16> {
        let b = self.bytes.get(self.at..self.at + 2)?;
        self.at += 2;
        Some(u16::from_be_bytes([b[0], b[1]]))
    }

    fn u4(&mut self) -> Option<u32> {
        let b = self.bytes.get(self.at..self.at + 4)?;
        self.at += 4;
        Some(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let b = self.bytes.get(self.at..self.at + n)?;
        self.at += n;
        Some(b)
    }
}

pub fn mod_dists(bytes: &[u8], annotation: &[u8]) -> Option<Vec<Vec<String>>> {
    let mut r = Reader { bytes, at: 0 };
    if r.u4()? != 0xCAFE_BABE {
        return None;
    }
    r.take(4)?;
    let count = r.u2()? as usize;
    let mut utf8: Vec<Option<&[u8]>> = vec![None; count];
    let mut i = 1;
    while i < count {
        let tag = r.u1()?;
        match tag {
            1 => {
                let len = r.u2()? as usize;
                utf8[i] = Some(r.take(len)?);
            }
            3 | 4 | 9 | 10 | 11 | 12 | 17 | 18 => {
                r.take(4)?;
            }
            5 | 6 => {
                r.take(8)?;
                i += 1;
            }
            7 | 8 | 16 | 19 | 20 => {
                r.take(2)?;
            }
            15 => {
                r.take(3)?;
            }
            _ => return None,
        }
        i += 1;
    }
    let text = |index: u16| utf8.get(index as usize).copied().flatten();
    r.take(6)?;
    let interfaces = r.u2()? as usize;
    r.take(interfaces * 2)?;
    for _ in 0..2 {
        let members = r.u2()?;
        for _ in 0..members {
            r.take(6)?;
            skip_attributes(&mut r)?;
        }
    }
    let mut found = Vec::new();
    let attributes = r.u2()?;
    for _ in 0..attributes {
        let name = text(r.u2()?)?;
        let len = r.u4()? as usize;
        let body = r.take(len)?;
        if name != b"RuntimeVisibleAnnotations" {
            continue;
        }
        let mut a = Reader { bytes: body, at: 0 };
        let annotations = a.u2()?;
        for _ in 0..annotations {
            let ty = text(a.u2()?)?;
            let pairs = a.u2()?;
            let mut dists = Vec::new();
            for _ in 0..pairs {
                let key = text(a.u2()?)?;
                if key == b"dist" {
                    collect_enums(&mut a, &text, &mut dists)?;
                } else {
                    skip_value(&mut a)?;
                }
            }
            if ty == annotation {
                found.push(dists);
            }
        }
    }
    Some(found)
}

fn skip_attributes(r: &mut Reader<'_>) -> Option<()> {
    let count = r.u2()?;
    for _ in 0..count {
        r.take(2)?;
        let len = r.u4()? as usize;
        r.take(len)?;
    }
    Some(())
}

fn collect_enums<'a>(
    r: &mut Reader<'_>,
    text: &impl Fn(u16) -> Option<&'a [u8]>,
    out: &mut Vec<String>,
) -> Option<()> {
    match r.u1()? {
        b'e' => {
            r.take(2)?;
            out.push(String::from_utf8_lossy(text(r.u2()?)?).into_owned());
        }
        b'[' => {
            let n = r.u2()?;
            for _ in 0..n {
                collect_enums(r, text, out)?;
            }
        }
        b'@' => {
            r.at -= 1;
            skip_value(r)?;
        }
        _ => {
            r.take(2)?;
        }
    }
    Some(())
}

fn skip_value(r: &mut Reader<'_>) -> Option<()> {
    match r.u1()? {
        b'e' => {
            r.take(4)?;
        }
        b'@' => {
            r.take(2)?;
            let pairs = r.u2()?;
            for _ in 0..pairs {
                r.take(2)?;
                skip_value(r)?;
            }
        }
        b'[' => {
            let n = r.u2()?;
            for _ in 0..n {
                skip_value(r)?;
            }
        }
        _ => {
            r.take(2)?;
        }
    }
    Some(())
}
