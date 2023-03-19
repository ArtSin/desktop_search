use ndarray::{Array2, ArrayD, ArrayViewD, Axis};

use tokenizers::{EncodeInput, Tokenizer};
use tracing_unwrap::{OptionExt, ResultExt};

pub struct PreprocessedText {
    pub input_ids: Array2<i64>,
    pub attention_mask: Array2<i64>,
    pub type_ids: Option<Array2<i64>>,
}

pub fn preprocess_texts<'a, T: Into<EncodeInput<'a>> + Send>(
    tokenizer: &Tokenizer,
    texts: Vec<T>,
    type_ids: bool,
) -> tokenizers::Result<PreprocessedText> {
    let encodings = tokenizer.encode_batch(texts, true)?;
    let sequence_length = encodings[0].get_ids().len();
    let input_ids: Vec<_> = encodings
        .iter()
        .flat_map(|a| a.get_ids().iter().map(|x| *x as i64).collect::<Vec<_>>())
        .collect();
    let attention_mask: Vec<_> = encodings
        .iter()
        .flat_map(|a| {
            a.get_attention_mask()
                .iter()
                .map(|x| *x as i64)
                .collect::<Vec<_>>()
        })
        .collect();
    let type_ids: Option<Vec<_>> = type_ids.then(|| {
        encodings
            .iter()
            .flat_map(|a| {
                a.get_type_ids()
                    .iter()
                    .map(|x| *x as i64)
                    .collect::<Vec<_>>()
            })
            .collect()
    });

    Ok(PreprocessedText {
        input_ids: Array2::from_shape_vec((encodings.len(), sequence_length), input_ids)
            .unwrap_or_log(),
        attention_mask: Array2::from_shape_vec((encodings.len(), sequence_length), attention_mask)
            .unwrap_or_log(),
        type_ids: type_ids
            .map(|x| Array2::from_shape_vec((encodings.len(), sequence_length), x).unwrap_or_log()),
    })
}

pub fn mean_pooling(
    last_hidden_state: &ArrayViewD<f32>,
    attention_mask: Array2<i64>,
) -> ArrayD<f32> {
    let input_mask_expanded = attention_mask
        .insert_axis(Axis(2))
        .broadcast(last_hidden_state.dim())
        .unwrap_or_log()
        .mapv(|x| x as f32);
    (last_hidden_state * &input_mask_expanded.view()).sum_axis(Axis(1))
        / (input_mask_expanded.sum_axis(Axis(1)).mapv(|x| x.max(1e-9)))
}
