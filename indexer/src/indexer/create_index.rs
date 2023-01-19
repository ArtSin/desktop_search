use axum::http::StatusCode;
use common_lib::elasticsearch::ELASTICSEARCH_INDEX;
use elasticsearch::{
    indices::{IndicesCreateParts, IndicesExistsParts},
    Elasticsearch,
};
use serde_json::json;

/// Creates index for storing indexed files, if it doesn't exist
pub async fn create_index(es_client: &Elasticsearch) -> Result<(), elasticsearch::Error> {
    // Check if index exists
    if es_client
        .indices()
        .exists(IndicesExistsParts::Index(&[ELASTICSEARCH_INDEX]))
        .send()
        .await?
        .status_code()
        == StatusCode::OK
    {
        return Ok(());
    }

    // Create index and set mapping
    es_client
        .indices()
        .create(IndicesCreateParts::Index(ELASTICSEARCH_INDEX))
        .body(json!({
            "settings": {
                "index": {
                    "analysis": {
                        "char_filter": {
                            "path_char_filter": {
                                "type": "mapping",
                                "mappings": ["_ => -", ". => -"]
                            }
                        },
                        "filter": {
                            "english_stemmer": {
                                "type": "stemmer",
                                "name": "english"
                            },
                            "russian_stemmer": {
                                "type": "stemmer",
                                "name": "russian"
                            },
                            "english_stop": {
                                "type": "stop",
                                "stopwords": "_english_"
                            },
                            "russian_stop": {
                                "type": "stop",
                                "stopwords": "_russian_"
                            },
                            "shingles": {
                                "type": "shingle",
                                "min_shingle_size": 2,
                                "max_shingle_size": 3
                            }
                        },
                        "analyzer": {
                            "en_ru_analyzer": {
                                "tokenizer": "standard",
                                "filter": [
                                    "lowercase",
                                    "english_stemmer",
                                    "russian_stemmer",
                                    "english_stop",
                                    "russian_stop"
                                ]
                            },
                            "path_en_ru_analyzer": {
                                "char_filter": "path_char_filter",
                                "tokenizer": "standard",
                                "filter": [
                                    "lowercase",
                                    "english_stemmer",
                                    "russian_stemmer",
                                    "english_stop",
                                    "russian_stop"
                                ]
                            },
                            "en_ru_analyzer_shingles": {
                                "tokenizer": "standard",
                                "filter": [
                                    "lowercase",
                                    "shingles"
                                ]
                            },
                            "path_en_ru_analyzer_shingles": {
                                "char_filter": "path_char_filter",
                                "tokenizer": "standard",
                                "filter": [
                                    "lowercase",
                                    "shingles"
                                ]
                            },
                        }
                    }
                }
            },
            "mappings": {
                "properties": {
                    "path": {
                        "type": "text",
                        "analyzer": "path_en_ru_analyzer",
                        "fields": {
                            "shingles": {
                                "type": "text",
                                "analyzer": "path_en_ru_analyzer_shingles"
                            }
                        }
                    },
                    "modified": {
                        "type": "long"
                    },
                    "size": {
                        "type": "long"
                    },
                    "hash": {
                        "type": "keyword"
                    },
                    "content_type": {
                        "type": "keyword"
                    },
                    "content_type_mime_type": {
                        "type": "keyword"
                    },
                    "content_type_mime_essence": {
                        "type": "keyword"
                    },
                    "content": {
                        "type": "text",
                        "analyzer": "en_ru_analyzer",
                        "fields": {
                            "shingles": {
                                "type": "text",
                                "analyzer": "en_ru_analyzer_shingles"
                            }
                        }
                    },

                    "text_embedding": {
                        "type": "dense_vector",
                        "dims": 384,
                        "index": true,
                        "similarity": "dot_product"
                    },

                    "image_embedding": {
                        "type": "dense_vector",
                        "dims": 512,
                        "index": true,
                        "similarity": "dot_product"
                    },
                    "width": {
                        "type": "integer"
                    },
                    "height": {
                        "type": "integer"
                    },

                    "title": {
                        "type": "text",
                        "analyzer": "en_ru_analyzer"
                    },
                    "creator": {
                        "type": "text",
                        "analyzer": "en_ru_analyzer"
                    },
                    "doc_created": {
                        "type": "long"
                    },
                    "doc_modified": {
                        "type": "long"
                    },
                    "num_pages": {
                        "type": "integer"
                    },
                    "num_words": {
                        "type": "integer"
                    },
                    "num_characters": {
                        "type": "integer"
                    }
                }
            }
        }))
        .send()
        .await?;
    Ok(())
}
