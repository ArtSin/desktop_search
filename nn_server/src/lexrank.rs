use ndarray::{Array, Array1, Array2, Axis};

pub fn degree_centrality_scores(similarity_matrix: Array2<f64>) -> Array1<f64> {
    power_method(create_markov_matrix(similarity_matrix))
}

fn create_markov_matrix(mut weights_matrix: Array2<f64>) -> Array2<f64> {
    let min_val = *weights_matrix.fold(weights_matrix.first().unwrap(), |acc, x| {
        if x < acc {
            x
        } else {
            acc
        }
    });
    if min_val <= 0.0 {
        // Use softmax
        weights_matrix.mapv_inplace(f64::exp);
    }
    let row_sum = weights_matrix.sum_axis(Axis(1)).insert_axis(Axis(1));
    weights_matrix / row_sum
}

fn power_method(transition_matrix: Array2<f64>) -> Array1<f64> {
    const MAX_ITER: usize = 10000;

    let mut eigenvector = Array::ones(transition_matrix.shape()[0]);
    if eigenvector.len() == 1 {
        return eigenvector;
    }

    let mut transition = transition_matrix.reversed_axes();
    for _ in 0..MAX_ITER {
        let eigenvector_next = transition.dot(&eigenvector);
        if eigenvector_next
            .as_slice()
            .unwrap()
            .iter()
            .zip(eigenvector.as_slice().unwrap())
            .all(|(x, y)| (x - y).abs() < 1e-8)
        {}
        if eigenvector_next.abs_diff_eq(&eigenvector, 1e-8) {
            return eigenvector_next;
        }
        eigenvector = eigenvector_next;
        transition = transition.dot(&transition);
    }
    eigenvector
}
