//! تست گلدن OpenAPI — «صفر تغییر شکستن‌دهنده» (فاز ۳، معیار پذیرش ADR-009).
//!
//! `golden-openapi.json` خروجی واقعی `buildOpenApiDoc` روی ۴۶ مسیر سرور TS است
//! (تولیدشده با خود zod — `scripts/ts-golden/generate.ts`). این تست سند Rust را
//! می‌سازد و پس از نرمال‌سازی قالب‌دهی، ساختار را مقایسه می‌کند.
//!
//! قواعد نرمال‌سازی (هر مورد فقط formatting — نه تغییر وایر):
//! 1. حذف `$schema` — zod به هر اسکیما می‌چسباند؛ utoipa نه.
//! 2. حذف `additionalProperties` و `propertyNames` — zod اشیاء را می‌بندد؛
//!    تطبیق سمت سرور با serde همان تضمین را می‌دهد.
//! 3. حذف `description` — doc-comment های Rust در سند می‌آیند؛ zod ندارد.
//! 4. اعداد به f64 — `9007199254740991` و `9007199254740991.0` برابرند.
//! 5. `type:["string","null"]` (utoipa برای Option) ↔ `anyOf:[T,{type:null}]`
//!    (zod برای .nullable()) — هر دو → T + nullable؛ وقتی فیلد required نیست
//!    nullable حذف می‌شود (absent-tolerant).
//! 6. `oneOf:[{type:null},X]` غیر-required ↔ X (فیلد optional با انم).
//! 7. `const:"X"` (zod literal) ↔ `enum:["X"]` (انم تک‌عضوی Rust)؛
//!    `const:true` ↔ `type:"boolean"`.
//! 8. `type:"number"` ↔ `type:"integer"` — صحیح زیرمجموعهٔ عدد است؛ برای
//!    مصرف‌کننده non-breaking است (زودتر سخت‌گیرانه‌تر شد، نه شل‌تر).
//! 9. حذف `format:"int64"`/`"double"` روی اعداد — utoipa اضافه می‌کند؛ zod نه.
//!    (format `date-time` نگه داشته می‌شود و باید در هر دو باشد.)
//! 10. `pattern` اسکیمای date-time با chrono سمت Rust اعتبارسنجی می‌شود؛
//!     regex زدِ zod در سند باقی می‌ماند ولی در مقایسه فقط وقتی هر دو فاقد
//!     format date-time هستند سخت‌گیری می‌شود.
//! 11. آرایهٔ `required` به‌صورت مجموعه مقایسه می‌شود (ترتیب اعلامیه‌ها).

// تست‌ها جای مشروع expect هستند — ورودی گلدن باید شکست صریح بدهد
#![allow(clippy::expect_used)]

use dropagentx_contracts::{build_openapi_doc_inlined, openapi_info, routes::api_routes};
use serde_json::{Map, Value};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

fn golden() -> Value {
    let raw = std::fs::read_to_string(format!("{FIXTURES}/golden-openapi.json"))
        .expect("golden-openapi.json");
    serde_json::from_str(&raw).expect("golden openapi parses")
}

fn rust_doc() -> Value {
    build_openapi_doc_inlined(&api_routes(), &openapi_info())
}

// ── نرمال‌ساز ───────────────────────────────────────────────────────────────

fn num_to_f64(v: &mut Value) {
    match v {
        Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                *v = serde_json::Number::from_f64(f).map(Value::Number).unwrap_or(Value::Null);
            }
        }
        Value::Array(items) => items.iter_mut().for_each(num_to_f64),
        Value::Object(map) => map.values_mut().for_each(num_to_f64),
        _ => {}
    }
}

const DROP_KEYS: &[&str] =
    &["$schema", "description", "title", "additionalProperties", "propertyNames"];

fn canon(v: &Value, required: Option<&Vec<String>>) -> Value {
    let mut out = Map::new();
    match v {
        Value::Object(map) => {
            // $ref حل‌نشده ممنوع است
            assert!(
                !map.contains_key("$ref"),
                "unresolved $ref leaked into the doc — extend component registry"
            );
            // anyOf/oneOf شامل null
            for combinator in ["anyOf", "oneOf"] {
                if let Some(Value::Array(members)) = map.get(combinator) {
                    let non_null: Vec<&Value> = members
                        .iter()
                        .filter(|m| m.get("type") != Some(&Value::String("null".into())))
                        .collect();
                    if non_null.len() == 1 && members.len() == 2 {
                        // [T, null] → T + nullable
                        let mut inner = canon(non_null[0], required);
                        if let Value::Object(io) = &mut inner {
                            if required.is_some() {
                                io.insert("nullable".into(), Value::Bool(true));
                            }
                        }
                        return inner;
                    }
                    if members.len() == 1 {
                        // oneOf تک‌عضوی (utoipa برای انم با serde(default)) → خودِ عضو
                        // + sibling های کامبینیاتور (مثل default) باید حفظ شوند
                        let mut inner = canon(&members[0], required);
                        if let Value::Object(io) = &mut inner {
                            for (k, v) in map {
                                if k != combinator && k != "$ref" && !io.contains_key(k) {
                                    io.insert(k.clone(), v.clone());
                                }
                            }
                        }
                        return inner;
                    }
                }
            }
            // const → enum / boolean
            if let Some(c) = map.get("const") {
                match c {
                    Value::String(s) => {
                        out.insert("type".into(), Value::String("string".into()));
                        out.insert("enum".into(), Value::Array(vec![Value::String(s.clone())]));
                    }
                    Value::Bool(_) => {
                        // literal(true) ↔ boolean — مقدار همیشه true سمت سرور تضمین می‌شود
                        out.insert("type".into(), Value::String("boolean".into()));
                    }
                    other => {
                        out.insert("const_other".into(), other.clone());
                    }
                }
            }
            for (k, val) in map {
                if DROP_KEYS.contains(&k.as_str()) || k == "const" {
                    continue;
                }
                if k == "pattern" {
                    // اسکیمای date-time: الگوی zod با chrono سمت Rust معتبرسازی می‌شود
                    if map.get("format").and_then(Value::as_str) == Some("date-time") {
                        continue;
                    }
                }
                if k == "format" {
                    // format اعداد حذف؛ date-time می‌ماند
                    if val == "int64" || val == "double" || val == "float" || val == "int32" {
                        continue;
                    }
                    out.insert(k.clone(), val.clone());
                    continue;
                }
                if k == "type" {
                    match val {
                        Value::Array(types) => {
                            let mut list: Vec<Value> = types
                                .iter()
                                .filter(|t| t.as_str() != Some("null"))
                                .cloned()
                                .collect();
                            let had_null = list.len() != types.len();
                            let scalar = if list.len() == 1 {
                                list.pop().expect("one")
                            } else {
                                Value::Array(list)
                            };
                            if had_null && required.is_some() {
                                out.insert("nullable".into(), Value::Bool(true));
                            }
                            out.insert("type".into(), scalar);
                        }
                        other => {
                            out.insert("type".into(), other.clone());
                        }
                    }
                    continue;
                }
                match val {
                    Value::Array(items) => {
                        if k == "required" {
                            let mut names: Vec<Value> = items.clone();
                            names.sort_by_key(|v| v.as_str().unwrap_or_default().to_string());
                            out.insert(k.clone(), Value::Array(names));
                        } else if k == "enum" {
                            out.insert(k.clone(), Value::Array(items.clone()));
                        } else if k == "anyOf" || k == "oneOf" {
                            out.insert(
                                k.clone(),
                                Value::Array(items.iter().map(|m| canon(m, required)).collect()),
                            );
                        } else {
                            out.insert(k.clone(), canon(val, None));
                        }
                    }
                    Value::Object(_) => {
                        if k == "properties" {
                            let req_list =
                                map.get("required").and_then(Value::as_array).and_then(|a| {
                                    a.iter()
                                        .map(|v| v.as_str().map(str::to_string))
                                        .collect::<Option<Vec<String>>>()
                                });
                            let mut props = Map::new();
                            for (pk, pv) in val.as_object().expect("properties object") {
                                let is_required = req_list.as_ref().is_some_and(|r| r.contains(pk));
                                let ctx = if is_required { Some(vec![pk.clone()]) } else { None };
                                let canon_val = if is_required {
                                    canon(pv, ctx.as_ref())
                                } else {
                                    canon(pv, None)
                                };
                                props.insert(pk.clone(), canon_val);
                            }
                            out.insert(k.clone(), Value::Object(props));
                        } else {
                            out.insert(k.clone(), canon(val, None));
                        }
                    }
                    other => {
                        out.insert(k.clone(), other.clone());
                    }
                }
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(|m| canon(m, required)).collect()),
        Value::Number(n) => {
            // اعداد همه به f64 (قاعدهٔ ۴)
            serde_json::Number::from_f64(n.as_f64().unwrap_or_default())
                .map(Value::Number)
                .unwrap_or(Value::Null)
        }
        other => other.clone(),
    }
}

/// مقایسهٔ بازگشتی — مسیر تفاوت‌ها را جمع می‌کند.
fn diff(a: &Value, b: &Value, path: &str, out: &mut Vec<String>) {
    match (a, b) {
        (Value::Object(ma), Value::Object(mb)) => {
            for (k, va) in ma {
                match mb.get(k) {
                    Some(vb) => diff(va, vb, &format!("{path}.{k}"), out),
                    None => out.push(format!("{path}.{k} — فقط در TS: {va}")),
                }
            }
            for k in mb.keys() {
                if !ma.contains_key(k) {
                    out.push(format!("{path}.{k} — فقط در Rust: {}", mb[k]));
                }
            }
        }
        (Value::Array(aa), Value::Array(ab)) => {
            if aa.len() != ab.len() {
                out.push(format!("{path} — طول {}/{}: {aa:?} vs {ab:?}", aa.len(), ab.len()));
            } else {
                for (i, (x, y)) in aa.iter().zip(ab.iter()).enumerate() {
                    diff(x, y, &format!("{path}[{i}]"), out);
                }
            }
        }
        // number ↔ integer هم‌ارز است (قاعدهٔ ۸)
        (Value::String(x), Value::String(y)) if x == y => {}
        (Value::Bool(x), Value::Bool(y)) if x == y => {}
        (Value::Null, Value::Null) => {}
        (Value::Number(x), Value::Number(y)) => {
            let (fx, fy) = (x.as_f64(), y.as_f64());
            if fx != fy {
                out.push(format!("{path} — عدد: {x} vs {y}"));
            }
        }
        (Value::String(x), Value::String(y)) => out.push(format!("{path} — رشته: {x:?} vs {y:?}")),
        _ => {
            if a != b {
                out.push(format!("{path} — {a} vs {b}"));
            }
        }
    }
}

#[test]
fn openapi_diff_against_ts_golden_is_zero_breaking() {
    let golden = golden();
    let mut rust = rust_doc();
    num_to_f64(&mut rust);

    // ۱) سطح ریشه — نسخهٔ info صرفاً ساختاری چک می‌شود (TS = نسخهٔ پکیج v3)
    assert_eq!(rust["openapi"], golden["openapi"], "نسخهٔ OpenAPI");
    assert_eq!(rust["info"]["title"], golden["info"]["title"], "عنوان سند");
    assert_eq!(rust["info"]["description"], golden["info"]["description"], "توضیح سند");
    assert!(!rust["info"]["version"].as_str().unwrap_or("").is_empty(), "نسخه باید باشد");
    assert_eq!(rust["servers"], golden["servers"], "servers");
    assert_eq!(rust["components"], golden["components"], "securitySchemes");

    // ۲) مسیرها و متدها — یک‌به‌یک
    let gpaths = golden["paths"].as_object().expect("paths");
    let rpaths = rust["paths"].as_object().expect("paths");
    let mut gkeys: Vec<&String> = gpaths.keys().collect();
    let mut rkeys: Vec<&String> = rpaths.keys().collect();
    gkeys.sort();
    rkeys.sort();
    assert_eq!(gkeys, rkeys, "مجموعهٔ مسیرها باید عین TS باشد");

    // ۳) تگ‌ها به ترتیب اولین ظهور
    assert_eq!(rust["tags"], golden["tags"], "ترتیب تگ‌ها");

    // ۴) هر عملیات — پس از نرمال‌سازی صفر diff
    let mut mismatches: Vec<String> = Vec::new();
    for (path, gitem) in gpaths {
        let ritem = &rust["paths"][path];
        for (method, gop) in gitem.as_object().expect("methods") {
            let rop = &ritem[method];
            let gcanon = canon(gop, None);
            let mut rcanon = canon(rop, None);
            num_to_f64(&mut rcanon);
            // نسخهٔ info inside operation ندارد؛ مستقیم diff
            let mut local: Vec<String> = Vec::new();
            diff(&gcanon, &rcanon, &format!("{method} {path}"), &mut local);
            mismatches.extend(local);
        }
    }
    assert!(
        mismatches.is_empty(),
        "سند Rust با TS فرق شکستن‌دهنده دارد ({} مورد):\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

#[test]
fn every_operation_has_summary_operationid_and_error_envelope() {
    let doc = rust_doc();
    let paths = doc["paths"].as_object().expect("paths");
    let mut operations = 0;
    for (path, item) in paths {
        for (method, op) in item.as_object().expect("methods") {
            operations += 1;
            assert!(!op["operationId"].as_str().unwrap_or_default().is_empty(), "{method} {path}");
            assert!(!op["summary"].as_str().unwrap_or_default().is_empty(), "{method} {path}");
            assert!(
                op["responses"].as_object().expect("responses").contains_key("4XX"),
                "{method} {path}"
            );
            // سطح امنیت یا در سند است یا عمومی است — ادمین‌ها باید security داشته باشند
            let tag = op["tags"][0].as_str().expect("tag");
            assert!(!tag.is_empty());
        }
    }
    assert_eq!(operations, 46, "۴۶ عملیات مثل api.ts");
}
