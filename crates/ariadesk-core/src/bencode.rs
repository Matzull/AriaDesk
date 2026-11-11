//! Minimal bencode decoder that keeps the raw byte span of every value,
//! so the info hash can be computed over the original `info` bytes.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value<'a> {
    Int(i64),
    Bytes(&'a [u8]),
    List(Vec<Node<'a>>),
    Dict(BTreeMap<&'a [u8], Node<'a>>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node<'a> {
    pub value: Value<'a>,
    pub raw: &'a [u8],
}

impl<'a> Node<'a> {
    pub fn get(&self, key: &str) -> Option<&Node<'a>> {
        match &self.value {
            Value::Dict(d) => d.get(key.as_bytes()),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self.value {
            Value::Int(i) => Some(i),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&'a [u8]> {
        match self.value {
            Value::Bytes(b) => Some(b),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<String> {
        self.as_bytes().map(|b| String::from_utf8_lossy(b).into_owned())
    }

    pub fn as_list(&self) -> Option<&[Node<'a>]> {
        match &self.value {
            Value::List(l) => Some(l),
            _ => None,
        }
    }
}

pub fn decode(data: &[u8]) -> Option<Node<'_>> {
    let (node, rest) = parse(data, 0)?;
    if rest != data.len() {
        return None;
    }
    Some(node)
}

const MAX_DEPTH: usize = 64;

fn parse(data: &[u8], pos: usize) -> Option<(Node<'_>, usize)> {
    parse_depth(data, pos, 0)
}

fn parse_depth(data: &[u8], start: usize, depth: usize) -> Option<(Node<'_>, usize)> {
    if depth > MAX_DEPTH {
        return None;
    }
    let mut pos = start;
    let value = match *data.get(pos)? {
        b'i' => {
            let end = pos + 1 + data[pos + 1..].iter().position(|&c| c == b'e')?;
            let n = std::str::from_utf8(&data[pos + 1..end]).ok()?.parse().ok()?;
            pos = end + 1;
            Value::Int(n)
        }
        b'l' => {
            pos += 1;
            let mut list = Vec::new();
            while *data.get(pos)? != b'e' {
                let (n, p) = parse_depth(data, pos, depth + 1)?;
                list.push(n);
                pos = p;
            }
            pos += 1;
            Value::List(list)
        }
        b'd' => {
            pos += 1;
            let mut dict = BTreeMap::new();
            while *data.get(pos)? != b'e' {
                let (k, p) = parse_depth(data, pos, depth + 1)?;
                let key = k.as_bytes()?;
                let (v, p) = parse_depth(data, p, depth + 1)?;
                dict.insert(key, v);
                pos = p;
            }
            pos += 1;
            Value::Dict(dict)
        }
        b'0'..=b'9' => {
            let colon = pos + data[pos..].iter().position(|&c| c == b':')?;
            let len: usize = std::str::from_utf8(&data[pos..colon]).ok()?.parse().ok()?;
            let begin = colon + 1;
            let end = begin.checked_add(len)?;
            if end > data.len() {
                return None;
            }
            pos = end;
            Value::Bytes(&data[begin..end])
        }
        _ => return None,
    };
    Some((Node { value, raw: &data[start..pos] }, pos))
}

/// Encoder used for building fixtures in tests.
#[cfg(test)]
pub mod enc {
    pub enum E {
        I(i64),
        B(Vec<u8>),
        L(Vec<E>),
        D(Vec<(&'static str, E)>),
    }

    pub fn s(v: &str) -> E {
        E::B(v.as_bytes().to_vec())
    }

    pub fn encode(e: &E, out: &mut Vec<u8>) {
        match e {
            E::I(i) => out.extend(format!("i{i}e").bytes()),
            E::B(b) => {
                out.extend(format!("{}:", b.len()).bytes());
                out.extend(b);
            }
            E::L(l) => {
                out.push(b'l');
                l.iter().for_each(|x| encode(x, out));
                out.push(b'e');
            }
            E::D(d) => {
                let mut d: Vec<_> = d.iter().collect();
                d.sort_by_key(|(k, _)| *k);
                out.push(b'd');
                for (k, v) in d {
                    encode(&s(k), out);
                    encode(v, out);
                }
                out.push(b'e');
            }
        }
    }
}
