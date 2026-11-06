//! Config edits from the visual pages: changes the TOML by path and keeps comments, formatting and
//! order (toml_edit).
//!
//! A path is a list of key names and indexes, such as `["device", "niz", "rule", 2, "enabled"]`.
//! After an edit the caller has kanami-config validate it, and the file is written only if it
//! passes.

use kanami_i18n::tr;
use serde::Deserialize;
use serde_json::{Map, Value as Json};
use toml_edit::{Array, ArrayOfTables, DocumentMut, InlineTable, Item, Table, Value};

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Seg {
    Index(usize),
    Key(String),
}

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    /// Sets a value, creating any missing tables on the path. Objects are written as inline tables by default, or as `[table]` when `table` is true.
    Set {
        path: Vec<Seg>,
        value: Json,
        #[serde(default)]
        table: bool,
    },
    /// Removes a key or array element; an emptied `[[array]]` is removed together with its subtables.
    Remove { path: Vec<Seg> },
    /// Inserts at `index` in an array, creating the array if it does not exist. An inserted object becomes a `[[table]]`.
    Insert {
        path: Vec<Seg>,
        index: usize,
        value: Json,
    },
    /// Moves an array element from `from` to `to`.
    Move {
        path: Vec<Seg>,
        from: usize,
        to: usize,
    },
    /// Moves a `[table]` (with its subtables) before or after another table, such as moving a rule
    /// before a group. Only its position in the file changes.
    Place {
        path: Vec<Seg>,
        anchor: Vec<Seg>,
        #[serde(default)]
        after: bool,
    },
    /// Renames a key in a table, keeping its position and content (such as `[device.niz]` to `[device.hhkb]`).
    Rename { path: Vec<Seg>, to: String },
}

pub fn apply(text: &str, ops: &[Op]) -> Result<String, String> {
    let mut doc: DocumentMut = text
        .parse()
        .map_err(|e| tr!("The config file is not valid TOML: {error}", error = e))?;
    for op in ops {
        apply_one(&mut doc, op)?;
    }
    Ok(doc.to_string())
}

fn apply_one(doc: &mut DocumentMut, op: &Op) -> Result<(), String> {
    match op {
        Op::Set { path, value, table } => {
            let (last, parent) = split(path)?;
            let parent = walk(doc.as_item_mut(), parent, true)?;
            let new = if *table {
                Item::Table(to_table(value)?)
            } else {
                Item::Value(to_value(value)?)
            };
            set(parent, last, new, path)
        }
        Op::Remove { path } => {
            let (last, parent_path) = split(path)?;
            let parent = walk(doc.as_item_mut(), parent_path, false)?;
            remove(parent, last).ok_or_else(|| missing(path))?;
            // An emptied `[[array]]` takes its subtables with it
            let empty = match &*parent {
                Item::ArrayOfTables(a) => a.is_empty(),
                Item::Table(t) => t.is_empty() && !parent_path.is_empty(),
                _ => false,
            };
            if empty && let Ok((key, grand)) = split(parent_path) {
                let grand = walk(doc.as_item_mut(), grand, false)?;
                remove(grand, key);
            }
            Ok(())
        }
        Op::Insert { path, index, value } => {
            let (last, parent) = split(path)?;
            let Seg::Key(key) = last else {
                return Err(tr!("{path} is not an array", path = show(path)));
            };
            let parent = walk(doc.as_item_mut(), parent, true)?;
            let table = parent
                .as_table_like_mut()
                .ok_or_else(|| tr!("{path} is not a table", path = show(path)))?;
            if table.get(key).is_none() {
                let empty = if value.is_object() {
                    Item::ArrayOfTables(ArrayOfTables::new())
                } else {
                    Item::Value(Value::Array(Array::new()))
                };
                table.insert(key, empty);
            }
            match table.get_mut(key) {
                Some(Item::ArrayOfTables(a)) => {
                    let mut new = to_table(value)?;
                    let index = (*index).min(a.len());
                    // When inserting at the front, borrow the position of the old first one; otherwise it is written after the previous one
                    if index == 0
                        && let Some(first) = a.get(0)
                    {
                        new.set_position(first.position());
                    }
                    a.insert(index, new);
                }
                Some(Item::Value(Value::Array(a))) => {
                    let index = (*index).min(a.len());
                    a.insert_formatted(index, to_value(value)?);
                    if index == 0 && a.len() > 1 {
                        // The new first element takes the leading whitespace of the old first one
                        let decor = a.get(1).map(|v| v.decor().clone());
                        if let Some(decor) = decor {
                            *a.get_mut(0).unwrap().decor_mut() = decor;
                            a.get_mut(1).unwrap().decor_mut().set_prefix(" ");
                        }
                    }
                }
                _ => return Err(tr!("{path} is not an array", path = show(path))),
            }
            Ok(())
        }
        Op::Move { path, from, to } => {
            let len = match walk(doc.as_item_mut(), path, false)? {
                Item::ArrayOfTables(a) => a.len(),
                Item::Value(Value::Array(a)) => a.len(),
                _ => return Err(tr!("{path} is not an array", path = show(path))),
            };
            if *from >= len || *to >= len {
                return Err(tr!(
                    "{path} has no item {number}",
                    path = show(path),
                    number = from.max(to) + 1
                ));
            }
            if from == to {
                return Ok(());
            }
            match walk(doc.as_item_mut(), path, false)? {
                // `[[array]]` is written by position, so moving means changing the position
                Item::ArrayOfTables(_) => {
                    let element = |i: usize| [path.as_slice(), &[Seg::Index(i)]].concat();
                    place(doc, &element(*from), &element(*to), to > from)?;
                }
                Item::Value(Value::Array(a)) => {
                    let v = a.remove(*from);
                    a.insert_formatted(*to, v);
                    a.fmt();
                }
                _ => unreachable!(),
            }
            Ok(())
        }
        Op::Place {
            path,
            anchor,
            after,
        } => place(doc, path, anchor, *after),
        Op::Rename { path, to } => {
            let (last, parent) = split(path)?;
            let Seg::Key(key) = last else {
                return Err(tr!("{path} is not a key", path = show(path)));
            };
            let table = walk(doc.as_item_mut(), parent, false)?
                .as_table_like_mut()
                .ok_or_else(|| missing(path))?;
            if table.contains_key(to) {
                return Err(tr!("There is already an entry named {name}", name = to));
            }
            let pairs: Vec<(String, Item)> = table
                .iter()
                .map(|(k, _)| k.to_owned())
                .collect::<Vec<_>>()
                .into_iter()
                .map(|k| {
                    let item = table.remove(&k).unwrap();
                    (if &k == key { to.clone() } else { k }, item)
                })
                .collect();
            if !pairs.iter().any(|(k, _)| k == to) {
                return Err(missing(path));
            }
            for (k, item) in pairs {
                table.insert(&k, item);
            }
            Ok(())
        }
    }
}

fn split(path: &[Seg]) -> Result<(&Seg, &[Seg]), String> {
    path.split_last()
        .ok_or_else(|| tr!("The path cannot be empty"))
}

fn show(path: &[Seg]) -> String {
    path.iter()
        .map(|s| match s {
            Seg::Index(i) => format!("[{i}]"),
            Seg::Key(k) => format!(".{k}"),
        })
        .collect::<String>()
        .trim_start_matches('.')
        .to_owned()
}

fn missing(path: &[Seg]) -> String {
    tr!(
        "The config has no {path}; it may have just been changed elsewhere",
        path = show(path)
    )
}

/// Walks down the path. With `create`, missing keys become tables (inline tables inside inline tables).
fn walk<'a>(mut item: &'a mut Item, path: &[Seg], create: bool) -> Result<&'a mut Item, String> {
    for (i, seg) in path.iter().enumerate() {
        item = match seg {
            Seg::Key(key) => {
                let inline = !item.is_table();
                let table = item
                    .as_table_like_mut()
                    .ok_or_else(|| tr!("{path} is not a table", path = show(&path[..i])))?;
                if create && table.get(key).is_none() {
                    let new = if inline {
                        Item::Value(Value::InlineTable(InlineTable::new()))
                    } else {
                        let mut t = Table::new();
                        t.set_implicit(true);
                        Item::Table(t)
                    };
                    table.insert(key, new);
                }
                table.get_mut(key).ok_or_else(|| missing(&path[..=i]))?
            }
            Seg::Index(index) => item.get_mut(*index).ok_or_else(|| missing(&path[..=i]))?,
        };
    }
    Ok(item)
}

fn set(parent: &mut Item, last: &Seg, mut new: Item, path: &[Seg]) -> Result<(), String> {
    let slot = match last {
        Seg::Key(key) => {
            let inline = !parent.is_table();
            let table = parent.as_table_like_mut().ok_or_else(|| {
                tr!(
                    "{path} is not a table",
                    path = show(&path[..path.len() - 1])
                )
            })?;
            if inline && let Item::Table(t) = new {
                new = Item::Value(Value::InlineTable(t.into_inline_table()));
            }
            match table.get_mut(key) {
                Some(slot) => slot,
                None => {
                    table.insert(key, new);
                    tidy(parent);
                    return Ok(());
                }
            }
        }
        Seg::Index(i) => parent.get_mut(*i).ok_or_else(|| missing(path))?,
    };
    // Keep the whitespace around the old value and its trailing comment
    match (&*slot, &mut new) {
        (Item::Value(old), Item::Value(v)) => *v.decor_mut() = old.decor().clone(),
        (Item::Table(old), Item::Table(t)) => {
            *t.decor_mut() = old.decor().clone();
            t.set_position(old.position());
        }
        _ => {}
    }
    *slot = new;
    Ok(())
}

/// Re-spaces an inline table after keys are added or removed: `{ a = 1, b = 2 }`.
fn tidy(item: &mut Item) {
    if let Some(t) = item.as_inline_table_mut() {
        t.fmt();
    }
}

fn remove(parent: &mut Item, last: &Seg) -> Option<Item> {
    match last {
        Seg::Key(key) => {
            let removed = parent.as_table_like_mut()?.remove(key);
            tidy(parent);
            removed
        }
        Seg::Index(i) => match parent {
            Item::ArrayOfTables(a) if *i < a.len() => Some(Item::Table(a.remove(*i))),
            Item::Value(Value::Array(a)) if *i < a.len() => {
                let v = a.remove(*i);
                a.fmt();
                Some(Item::Value(v))
            }
            _ => None,
        },
    }
}

/// Moves table `path` with its subtables (such as `[[group.rule]]`) before or after table `anchor`
/// (with its subtables).
///
/// toml_edit writes tables by the position recorded on each one, so this first works out the
/// current order of all table headers in the file, moves this block over, and renumbers them in the
/// new order. When the file is read again, the array order is the order in the file.
fn place(doc: &mut DocumentMut, path: &[Seg], anchor: &[Seg], after: bool) -> Result<(), String> {
    if anchor.starts_with(path) {
        return Err(tr!("Cannot move something into itself"));
    }
    let mut order = Vec::new();
    let mut last = 0;
    visit(doc.as_table(), &mut Vec::new(), &mut |table, p| {
        if let Some(pos) = table.position() {
            last = pos;
        }
        order.push((last, p.to_vec()));
    });
    order.sort_by_key(|(pos, p)| (!p.is_empty(), *pos));
    let mut order: Vec<Vec<Seg>> = order.into_iter().map(|(_, p)| p).collect();

    let block: Vec<Vec<Seg>> = order
        .iter()
        .filter(|p| p.starts_with(path))
        .cloned()
        .collect();
    if block.is_empty() {
        return Err(missing(path));
    }
    order.retain(|p| !p.starts_with(path));
    let at = if after {
        order
            .iter()
            .rposition(|p| p.starts_with(anchor))
            .map(|i| i + 1)
    } else {
        order.iter().position(|p| p.starts_with(anchor))
    }
    .ok_or_else(|| missing(anchor))?;
    order.splice(at..at, block);

    for (n, p) in order.iter().enumerate() {
        if let Item::Table(t) = walk(doc.as_item_mut(), p, false)? {
            t.set_position(Some(n as isize));
        }
    }
    Ok(())
}

fn visit(table: &Table, path: &mut Vec<Seg>, f: &mut impl FnMut(&Table, &[Seg])) {
    if !table.is_dotted() {
        f(table, path);
    }
    for (key, item) in table.iter() {
        match item {
            Item::Table(t) => {
                path.push(Seg::Key(key.to_owned()));
                visit(t, path, f);
                path.pop();
            }
            Item::ArrayOfTables(a) => {
                for (i, t) in a.iter().enumerate() {
                    path.push(Seg::Key(key.to_owned()));
                    path.push(Seg::Index(i));
                    visit(t, path, f);
                    path.pop();
                    path.pop();
                }
            }
            _ => {}
        }
    }
}

// ---- Converting between JSON and TOML ----

fn to_value(json: &Json) -> Result<Value, String> {
    Ok(match json {
        Json::String(s) => s.as_str().into(),
        Json::Bool(b) => (*b).into(),
        Json::Number(n) => match n.as_i64() {
            Some(i) => i.into(),
            None => n.as_f64().unwrap_or_default().into(),
        },
        Json::Array(items) => {
            let mut a = Array::new();
            for item in items {
                a.push(to_value(item)?);
            }
            Value::Array(a)
        }
        Json::Object(map) => {
            let mut t = InlineTable::new();
            for (k, v) in map {
                t.insert(k, to_value(v)?);
            }
            Value::InlineTable(t)
        }
        Json::Null => return Err(tr!("The value cannot be empty")),
    })
}

/// Writes an object as a `[table]`: arrays of objects inside it become `[[subtables]]`, other objects inline tables.
fn to_table(json: &Json) -> Result<Table, String> {
    let Json::Object(map) = json else {
        return Err(tr!("A table is needed here"));
    };
    let mut t = Table::new();
    for (k, v) in map {
        let item = match v {
            Json::Array(items) if !items.is_empty() && items.iter().all(Json::is_object) => {
                let mut a = ArrayOfTables::new();
                for item in items {
                    a.push(to_table(item)?);
                }
                Item::ArrayOfTables(a)
            }
            _ => Item::Value(to_value(v)?),
        };
        t.insert(k, item);
    }
    Ok(t)
}

/// Converts the config file to JSON for the UI, keeping the written order. Every `[table]` gets a `$line` (the line of its header).
pub fn model(text: &str) -> Result<Json, String> {
    let doc = toml_edit::Document::parse(text).map_err(|e| e.to_string())?;
    let line = |start: usize| text[..start.min(text.len())].matches('\n').count() + 1;
    Ok(item_json(doc.as_item(), &line))
}

fn item_json(item: &Item, line: &dyn Fn(usize) -> usize) -> Json {
    match item {
        Item::None => Json::Null,
        Item::Value(v) => value_json(v),
        Item::Table(t) => {
            let mut map = Map::new();
            if let Some(span) = t.span() {
                map.insert("$line".into(), line(span.start).into());
            }
            for (k, v) in t.iter() {
                map.insert(k.to_owned(), item_json(v, line));
            }
            Json::Object(map)
        }
        Item::ArrayOfTables(a) => Json::Array(
            a.iter()
                .map(|t| item_json(&Item::Table(t.clone()), line))
                .collect(),
        ),
    }
}

fn value_json(value: &Value) -> Json {
    match value {
        Value::String(s) => s.value().as_str().into(),
        Value::Integer(i) => (*i.value()).into(),
        Value::Float(f) => (*f.value()).into(),
        Value::Boolean(b) => (*b.value()).into(),
        Value::Datetime(d) => d.value().to_string().into(),
        Value::Array(a) => Json::Array(a.iter().map(value_json).collect()),
        Value::InlineTable(t) => Json::Object(
            t.iter()
                .map(|(k, v)| (k.to_owned(), value_json(v)))
                .collect(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(text: &str, ops: Json) -> String {
        let ops: Vec<Op> = serde_json::from_value(ops).unwrap();
        apply(text, &ops).unwrap()
    }

    const SAMPLE: &str = r#"# Leading comment
[settings]
tap_timeout = 200 # milliseconds

[[rule]]
from = "a"
to = "b"

[[group]]
name = "g"

[[group.rule]]
from = "c"
to = "d"

[[rule]]
from = "e"
to = "f"
"#;

    #[test]
    fn set_keeps_comments() {
        let out = run(
            SAMPLE,
            json!([{ "op": "set", "path": ["settings", "tap_timeout"], "value": 180 }]),
        );
        assert!(out.contains("tap_timeout = 180 # milliseconds"), "{out}");
        assert!(out.starts_with("# Leading comment"));
        let out = run(
            SAMPLE,
            json!([{ "op": "set", "path": ["rule", 1, "enabled"], "value": false }]),
        );
        assert!(out.contains("to = \"f\"\nenabled = false"), "{out}");
    }

    #[test]
    fn set_creates_tables() {
        let out = run(
            "",
            json!([{ "op": "set", "path": ["device", "niz", "remap", "caps_lock"], "value": "escape" }]),
        );
        assert_eq!(out.trim(), "[device.niz.remap]\ncaps_lock = \"escape\"");
        let out = run(
            "",
            json!([{ "op": "set", "path": ["device", "niz"], "value": { "product": "NIZ" }, "table": true }]),
        );
        assert_eq!(out.trim(), "[device.niz]\nproduct = \"NIZ\"");
    }

    #[test]
    fn insert_and_remove_tables() {
        let out = run(
            SAMPLE,
            json!([{ "op": "insert", "path": ["rule"], "index": 1, "value": { "from": "x", "to": "y" } }]),
        );
        let x = out.find("from = \"x\"").unwrap();
        assert!(
            out.find("from = \"a\"").unwrap() < x && x < out.find("[[group]]").unwrap(),
            "{out}"
        );

        let out = run(
            SAMPLE,
            json!([{ "op": "insert", "path": ["rule"], "index": 0, "value": { "from": "x" } }]),
        );
        assert!(
            out.find("from = \"x\"").unwrap() < out.find("from = \"a\"").unwrap(),
            "{out}"
        );

        let out = run(
            SAMPLE,
            json!([{ "op": "remove", "path": ["group", 0, "rule", 0] }]),
        );
        assert!(!out.contains("group.rule"), "{out}");
        let out = run(SAMPLE, json!([{ "op": "remove", "path": ["group", 0] }]));
        assert!(!out.contains("group"), "{out}");
    }

    #[test]
    fn move_carries_subtables() {
        let text = format!("{SAMPLE}\n[[group]]\nname = \"h\"\n");
        let out = run(
            &text,
            json!([{ "op": "move", "path": ["group"], "from": 0, "to": 1 }]),
        );
        let h = out.find("name = \"h\"").unwrap();
        let g = out.find("name = \"g\"").unwrap();
        let c = out.find("from = \"c\"").unwrap();
        assert!(h < g && g < c, "{out}");
        // Move it back: g and its subtables return to before h
        let back = run(
            &out,
            json!([{ "op": "move", "path": ["group"], "from": 1, "to": 0 }]),
        );
        let h = back.find("name = \"h\"").unwrap();
        assert!(back.find("name = \"g\"").unwrap() < back.find("from = \"c\"").unwrap());
        assert!(back.find("from = \"c\"").unwrap() < h, "{back}");

        // A rule moves before the group
        let out = run(
            SAMPLE,
            json!([{ "op": "place", "path": ["rule", 1], "anchor": ["group", 0] }]),
        );
        let e = out.find("from = \"e\"").unwrap();
        assert!(e < out.find("[[group]]").unwrap(), "{out}");
        assert!(
            out.find("[[group]]").unwrap() < out.find("from = \"c\"").unwrap(),
            "{out}"
        );

        let out = run(
            SAMPLE,
            json!([{ "op": "move", "path": ["rule"], "from": 1, "to": 0 }]),
        );
        assert!(
            out.find("from = \"e\"").unwrap() < out.find("from = \"a\"").unwrap(),
            "{out}"
        );
    }

    #[test]
    fn new_device_with_rule() {
        let out = run(
            SAMPLE,
            json!([
                { "op": "set", "path": ["device", "niz"], "value": { "product": "NIZ" }, "table": true },
                { "op": "insert", "path": ["device", "niz", "rule"], "index": 0,
                  "value": { "from": "x", "to": "y", "when": { "app": ["com.apple.Terminal"] } } },
                { "op": "set", "path": ["device", "niz", "remap", "f1"], "value": "none" },
            ]),
        );
        assert!(out.contains("[device.niz]\nproduct = \"NIZ\""), "{out}");
        assert!(out.contains("[[device.niz.rule]]\nfrom = \"x\"\nto = \"y\"\nwhen = { app = [\"com.apple.Terminal\"] }"), "{out}");
        assert!(out.contains("[device.niz.remap]\nf1 = \"none\""), "{out}");
        let parsed = model(&out).unwrap();
        assert_eq!(
            parsed["device"]["niz"]["rule"][0]["when"]["app"][0],
            "com.apple.Terminal"
        );
        let renamed = run(
            &out,
            json!([{ "op": "rename", "path": ["device", "niz"], "to": "hhkb" }]),
        );
        assert!(
            renamed.contains("[[device.hhkb.rule]]") && !renamed.contains("niz\n"),
            "{renamed}"
        );
    }

    #[test]
    fn inline_tables_and_arrays() {
        let text = "[layer.nav]\nhold = \"space\"\nmap = { h = \"left\", j = \"down\" }\n";
        let out = run(
            text,
            json!([{ "op": "set", "path": ["layer", "nav", "map", "k"], "value": "up" }]),
        );
        assert!(
            out.contains("map = { h = \"left\", j = \"down\", k = \"up\" }"),
            "{out}"
        );
        let out = run(
            text,
            json!([{ "op": "remove", "path": ["layer", "nav", "map", "h"] }]),
        );
        assert!(out.contains("map = { j = \"down\" }"), "{out}");
        let out = run(
            text,
            json!([{ "op": "rename", "path": ["layer", "nav"], "to": "arrows" }]),
        );
        assert!(out.contains("[layer.arrows]"), "{out}");
    }

    #[test]
    fn model_has_lines() {
        let m = model(SAMPLE).unwrap();
        assert_eq!(m["rule"][1]["$line"], 16);
        assert_eq!(m["group"][0]["rule"][0]["to"], "d");
        assert_eq!(m["settings"]["tap_timeout"], 200);
    }
}
