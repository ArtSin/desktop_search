use serde::Serialize;
use serde_json::{json, Value};

pub fn simple_query_string(mut query: String, fields: &[&str]) -> Value {
    if query.is_empty() {
        query = "*".to_owned();
    }
    json!({
        "simple_query_string": {
            "query": query,
            "fields": fields,
        }
    })
}

pub fn terms(field: &str, values: impl Serialize) -> Value {
    json!({
        "terms": {
            field: values
        }
    })
}

pub fn term(field: &str, value: impl Serialize) -> Value {
    json!({
        "term": {
            field: {
                "value": value,
            }
        }
    })
}

// pub fn match_(field: &str, query: impl Serialize) -> Value {
//     json!({
//         "match": {
//             field: {
//                 "query": query,
//             }
//         }
//     })
// }

pub fn range(field: &str, gte: impl Serialize, lte: impl Serialize) -> Value {
    json!({
        "range": {
            field: {
                "gte": gte,
                "lte": lte,
            }
        }
    })
}

pub fn suggest(mut query: String, main_field: &str, all_fields: &[&str]) -> Value {
    if query.is_empty() {
        query = "*".to_owned();
    }

    let generators: Vec<_> = all_fields
        .iter()
        .map(|x| {
            json!({
                "field": x,
                "suggest_mode": "missing"
            })
        })
        .collect();

    json!({
        "text": query,
        "simple_phrase": {
            "phrase": {
                "field": main_field,
                "size": 1,
                "gram_size": 3,
                "direct_generator": generators,
                "highlight": {
                    "pre_tag": "<i>",
                    "post_tag": "</i>"
                }
            }
        }
    })
}
