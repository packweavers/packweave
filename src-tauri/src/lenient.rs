use serde::de::DeserializeOwned;
use serde_json::{Map, Number, Value};

pub fn has_conflict(text: &str) -> bool {
    text.lines()
        .any(|l| l.starts_with("<<<<<<< ") || l.starts_with(">>>>>>> ") || l == "=======")
}

pub fn ours(text: &str) -> String {
    let mut out = Vec::new();
    let mut side = 0;
    for line in text.lines() {
        if line.starts_with("<<<<<<< ") {
            side = 1;
        } else if line == "=======" && side == 1 {
            side = 2;
        } else if line.starts_with(">>>>>>> ") && side == 2 {
            side = 0;
        } else if side != 2 {
            out.push(line);
        }
    }
    out.join("\n")
}

pub fn parse(text: &str) -> Option<(Value, bool)> {
    let text = text.trim_start_matches('\u{feff}');
    if let Ok(value) = serde_json::from_str::<Value>(text) {
        return Some((value, false));
    }
    let mut loose = Loose {
        s: text.chars().collect(),
        i: 0,
    };
    loose.value().map(|v| (v, true))
}

pub fn repair<T: DeserializeOwned>(text: &str, base: Value) -> Option<(T, bool)> {
    let (value, rewritten) = parse(text)?;
    if !value.is_object() {
        return None;
    }
    let (item, salvaged) = salvage::<T>(value, base)?;
    Some((item, rewritten || salvaged))
}

pub fn salvage<T: DeserializeOwned>(value: Value, base: Value) -> Option<(T, bool)> {
    if let Ok(item) = serde_json::from_value::<T>(value.clone()) {
        return Some((item, false));
    }
    let mut working = base;
    if !valid::<T>(&working) {
        return None;
    }
    if let Value::Object(map) = value {
        for (key, item) in map {
            graft::<T>(&mut working, &[key], &item);
        }
    }
    serde_json::from_value::<T>(working)
        .ok()
        .map(|item| (item, true))
}

fn valid<T: DeserializeOwned>(value: &Value) -> bool {
    serde_json::from_value::<T>(value.clone()).is_ok()
}

fn slot<'a>(root: &'a mut Value, path: &[String]) -> Option<&'a mut Map<String, Value>> {
    let mut node = root;
    for key in &path[..path.len() - 1] {
        node = node.as_object_mut()?.get_mut(key)?;
    }
    node.as_object_mut()
}

fn graft<T: DeserializeOwned>(root: &mut Value, path: &[String], incoming: &Value) {
    let key = path[path.len() - 1].clone();
    let Some(parent) = slot(root, path) else {
        return;
    };
    let previous = parent.insert(key.clone(), incoming.clone());
    if valid::<T>(root) {
        return;
    }
    let restore = |root: &mut Value, value: Option<Value>| {
        if let Some(parent) = slot(root, path) {
            match value {
                Some(v) => {
                    parent.insert(key.clone(), v);
                }
                None => {
                    parent.remove(&key);
                }
            }
        }
    };
    match incoming {
        Value::Object(map) => {
            let fresh = match &previous {
                Some(Value::Object(_)) => previous.clone(),
                _ => Some(Value::Object(Map::new())),
            };
            restore(root, fresh);
            if !valid::<T>(root) {
                restore(root, previous);
                return;
            }
            for (child, value) in map {
                let mut next = path.to_vec();
                next.push(child.clone());
                graft::<T>(root, &next, value);
            }
        }
        Value::Array(items) => {
            restore(root, Some(Value::Array(Vec::new())));
            if !valid::<T>(root) {
                restore(root, previous);
                return;
            }
            for item in items {
                if let Some(Value::Array(list)) = slot(root, path).and_then(|p| p.get_mut(&key)) {
                    list.push(item.clone());
                }
                if !valid::<T>(root) {
                    if let Some(Value::Array(list)) = slot(root, path).and_then(|p| p.get_mut(&key))
                    {
                        list.pop();
                    }
                }
            }
        }
        _ => restore(root, previous),
    }
}

struct Loose {
    s: Vec<char>,
    i: usize,
}

impl Loose {
    fn peek(&self) -> Option<char> {
        self.s.get(self.i).copied()
    }

    fn ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() || c == '\u{feff}' {
                self.i += 1;
            } else if c == '#' || (c == '/' && self.s.get(self.i + 1) == Some(&'/')) {
                while self.peek().is_some_and(|c| c != '\n') {
                    self.i += 1;
                }
            } else if c == '/' && self.s.get(self.i + 1) == Some(&'*') {
                self.i += 2;
                while self.i < self.s.len()
                    && !(self.s[self.i] == '*' && self.s.get(self.i + 1) == Some(&'/'))
                {
                    self.i += 1;
                }
                self.i = (self.i + 2).min(self.s.len());
            } else {
                return;
            }
        }
    }

    fn value(&mut self) -> Option<Value> {
        self.ws();
        match self.peek()? {
            '{' => {
                self.i += 1;
                Some(self.object())
            }
            '[' => {
                self.i += 1;
                Some(self.array())
            }
            '"' | '\'' => Some(Value::String(self.string())),
            c if c == '-' || c == '+' || c == '.' || c.is_ascii_digit() => Some(self.number()),
            c if c.is_alphanumeric() || c == '_' => Some(self.word()),
            _ => None,
        }
    }

    fn object(&mut self) -> Value {
        let mut map = Map::new();
        loop {
            self.ws();
            let Some(c) = self.peek() else {
                return Value::Object(map);
            };
            match c {
                '}' => {
                    self.i += 1;
                    return Value::Object(map);
                }
                ',' | ';' => {
                    self.i += 1;
                    continue;
                }
                ']' => {
                    self.i += 1;
                    continue;
                }
                _ => {}
            }
            let before = self.i;
            let key = if c == '"' || c == '\'' {
                self.string()
            } else {
                self.bare(&[':', '=', ',', '}'])
            };
            self.ws();
            if matches!(self.peek(), Some(':') | Some('=')) {
                self.i += 1;
            }
            if let Some(value) = self.value() {
                if !key.is_empty() {
                    map.insert(key, value);
                }
            }
            if self.i == before {
                self.i += 1;
            }
        }
    }

    fn array(&mut self) -> Value {
        let mut list = Vec::new();
        loop {
            self.ws();
            let Some(c) = self.peek() else {
                return Value::Array(list);
            };
            match c {
                ']' => {
                    self.i += 1;
                    return Value::Array(list);
                }
                ',' | ';' => {
                    self.i += 1;
                    continue;
                }
                '}' => {
                    self.i += 1;
                    continue;
                }
                _ => {}
            }
            let before = self.i;
            if let Some(value) = self.value() {
                list.push(value);
            }
            if self.i == before {
                self.i += 1;
            }
        }
    }

    fn string(&mut self) -> String {
        let quote = self.s[self.i];
        self.i += 1;
        let mut out = String::new();
        while let Some(c) = self.peek() {
            self.i += 1;
            if c == quote {
                return out;
            }
            if c == '\n' && quote == '\'' {
                return out;
            }
            if c != '\\' {
                out.push(c);
                continue;
            }
            let Some(e) = self.peek() else {
                break;
            };
            self.i += 1;
            match e {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                'r' => out.push('\r'),
                'b' => out.push('\u{8}'),
                'f' => out.push('\u{c}'),
                'u' => {
                    let hex: String = self.s[self.i..(self.i + 4).min(self.s.len())]
                        .iter()
                        .collect();
                    match u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                        Some(ch) => {
                            out.push(ch);
                            self.i += 4;
                        }
                        None => out.push('u'),
                    }
                }
                other => out.push(other),
            }
        }
        out
    }

    fn bare(&mut self, stops: &[char]) -> String {
        let start = self.i;
        while let Some(c) = self.peek() {
            if c.is_whitespace() || stops.contains(&c) || c == '{' || c == '[' {
                break;
            }
            self.i += 1;
        }
        self.s[start..self.i].iter().collect()
    }

    fn number(&mut self) -> Value {
        let raw = self.bare(&[',', '}', ']', ':']);
        if let Ok(n) = raw.parse::<i64>() {
            return Value::Number(n.into());
        }
        match raw.parse::<f64>().ok().and_then(Number::from_f64) {
            Some(n) => Value::Number(n),
            None => Value::String(raw),
        }
    }

    fn word(&mut self) -> Value {
        let raw = self.bare(&[',', '}', ']', ':', '=']);
        match raw.to_lowercase().as_str() {
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            "null" => Value::Null,
            _ => Value::String(raw),
        }
    }
}
